use crate::domain::{TaskRecord, TaskSpec, TaskStatus};
use crate::error::{HarnessError, HarnessResult};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskGraph {
    pub tasks: Vec<TaskSpec>,
}

impl TaskGraph {
    pub fn new(tasks: Vec<TaskSpec>) -> HarnessResult<Self> {
        let graph = Self { tasks };
        graph.validate()?;
        Ok(graph)
    }

    pub fn validate(&self) -> HarnessResult<()> {
        if self.tasks.is_empty() {
            return Err(HarnessError::Validation(
                "task graph must contain at least one task".into(),
            ));
        }
        if self.tasks.len() > 1_000 {
            return Err(HarnessError::Validation(
                "task graph exceeds 1,000 tasks".into(),
            ));
        }
        let ids: BTreeSet<&str> = self.tasks.iter().map(|task| task.id.as_str()).collect();
        if ids.len() != self.tasks.len() || ids.contains("") {
            return Err(HarnessError::Validation(
                "task ids must be unique and non-empty".into(),
            ));
        }
        for task in &self.tasks {
            if task.description.trim().is_empty() {
                return Err(HarnessError::Validation(format!(
                    "task {} has no description",
                    task.id
                )));
            }
            if let Some(parent) = task.parent_task_id.as_deref() {
                if parent == task.id || !ids.contains(parent) {
                    return Err(HarnessError::Validation(format!(
                        "task {} has an invalid parent",
                        task.id
                    )));
                }
            }
            for dependency in &task.dependencies {
                if dependency == &task.id || !ids.contains(dependency.as_str()) {
                    return Err(HarnessError::Validation(format!(
                        "task {} has invalid dependency {}",
                        task.id, dependency
                    )));
                }
            }
        }

        let mut indegree: BTreeMap<&str, usize> = ids.iter().map(|id| (*id, 0)).collect();
        let mut dependents: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for task in &self.tasks {
            let unique_dependencies: BTreeSet<&str> =
                task.dependencies.iter().map(String::as_str).collect();
            *indegree.get_mut(task.id.as_str()).expect("known task") = unique_dependencies.len();
            for dependency in unique_dependencies {
                dependents
                    .entry(dependency)
                    .or_default()
                    .push(task.id.as_str());
            }
        }
        let mut queue: VecDeque<&str> = indegree
            .iter()
            .filter_map(|(id, degree)| (*degree == 0).then_some(*id))
            .collect();
        let mut visited = 0;
        while let Some(id) = queue.pop_front() {
            visited += 1;
            for dependent in dependents.get(id).into_iter().flatten() {
                let degree = indegree.get_mut(dependent).expect("known dependent");
                *degree -= 1;
                if *degree == 0 {
                    queue.push_back(dependent);
                }
            }
        }
        if visited != self.tasks.len() {
            return Err(HarnessError::Validation(
                "task dependencies contain a cycle".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerSnapshot {
    pub tasks: Vec<TaskRecord>,
    pub running: usize,
    pub concurrency_limit: usize,
    pub terminal: bool,
}

pub struct Scheduler {
    tasks: BTreeMap<String, TaskRecord>,
    concurrency_limit: usize,
}

impl Scheduler {
    pub fn new(graph: TaskGraph, concurrency_limit: usize) -> HarnessResult<Self> {
        if concurrency_limit == 0 || concurrency_limit > 64 {
            return Err(HarnessError::Validation(
                "concurrency limit must be between 1 and 64".into(),
            ));
        }
        let now = Utc::now();
        let tasks = graph
            .tasks
            .into_iter()
            .map(|spec| {
                let id = spec.id.clone();
                (
                    id,
                    TaskRecord {
                        spec,
                        status: TaskStatus::Pending,
                        retry_count: 0,
                        created_at: now,
                        started_at: None,
                        completed_at: None,
                    },
                )
            })
            .collect();
        let mut scheduler = Self {
            tasks,
            concurrency_limit,
        };
        scheduler.refresh();
        Ok(scheduler)
    }

    pub fn ready(&self) -> Vec<&TaskRecord> {
        let capacity = self.concurrency_limit.saturating_sub(self.running_count());
        let mut ready: Vec<&TaskRecord> = self
            .tasks
            .values()
            .filter(|task| task.status == TaskStatus::Ready)
            .collect();
        ready.sort_by(|left, right| {
            right
                .spec
                .priority
                .cmp(&left.spec.priority)
                .then_with(|| left.spec.id.cmp(&right.spec.id))
        });
        ready.truncate(capacity);
        ready
    }

    pub fn start(&mut self, task_id: &str) -> HarnessResult<()> {
        if self.running_count() >= self.concurrency_limit {
            return Err(HarnessError::Conflict(
                "scheduler concurrency limit reached".into(),
            ));
        }
        let task = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| HarnessError::NotFound(format!("task {task_id}")))?;
        if task.status != TaskStatus::Ready {
            return Err(HarnessError::Conflict(format!(
                "task {task_id} is not ready"
            )));
        }
        task.status = TaskStatus::Running;
        task.started_at = Some(Utc::now());
        Ok(())
    }

    pub fn succeed(&mut self, task_id: &str) -> HarnessResult<()> {
        self.finish(task_id, true)
    }

    pub fn fail(&mut self, task_id: &str) -> HarnessResult<()> {
        let task = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| HarnessError::NotFound(format!("task {task_id}")))?;
        if task.status != TaskStatus::Running {
            return Err(HarnessError::Conflict(format!(
                "task {task_id} is not running"
            )));
        }
        if task.retry_count < task.spec.max_retries {
            task.retry_count += 1;
            task.status = TaskStatus::Pending;
            task.started_at = None;
        } else {
            task.status = TaskStatus::Failed;
            task.completed_at = Some(Utc::now());
        }
        self.refresh();
        Ok(())
    }

    pub fn cancel_all(&mut self) {
        let now = Utc::now();
        for task in self.tasks.values_mut() {
            if !matches!(
                task.status,
                TaskStatus::Succeeded | TaskStatus::Failed | TaskStatus::Cancelled
            ) {
                task.status = TaskStatus::Cancelled;
                task.completed_at = Some(now);
            }
        }
    }

    pub fn snapshot(&self) -> SchedulerSnapshot {
        SchedulerSnapshot {
            tasks: self.tasks.values().cloned().collect(),
            running: self.running_count(),
            concurrency_limit: self.concurrency_limit,
            terminal: self.tasks.values().all(|task| {
                matches!(
                    task.status,
                    TaskStatus::Succeeded
                        | TaskStatus::Failed
                        | TaskStatus::Cancelled
                        | TaskStatus::Blocked
                )
            }),
        }
    }

    fn finish(&mut self, task_id: &str, success: bool) -> HarnessResult<()> {
        let task = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| HarnessError::NotFound(format!("task {task_id}")))?;
        if task.status != TaskStatus::Running {
            return Err(HarnessError::Conflict(format!(
                "task {task_id} is not running"
            )));
        }
        task.status = if success {
            TaskStatus::Succeeded
        } else {
            TaskStatus::Failed
        };
        task.completed_at = Some(Utc::now());
        self.refresh();
        Ok(())
    }

    fn refresh(&mut self) {
        let statuses: BTreeMap<String, TaskStatus> = self
            .tasks
            .iter()
            .map(|(id, task)| (id.clone(), task.status))
            .collect();
        for task in self.tasks.values_mut() {
            if task.status != TaskStatus::Pending {
                continue;
            }
            let dependency_statuses: Vec<TaskStatus> = task
                .spec
                .dependencies
                .iter()
                .filter_map(|id| statuses.get(id).copied())
                .collect();
            if dependency_statuses.iter().any(|status| {
                matches!(
                    status,
                    TaskStatus::Failed | TaskStatus::Cancelled | TaskStatus::Blocked
                )
            }) {
                task.status = TaskStatus::Blocked;
                task.completed_at = Some(Utc::now());
            } else if dependency_statuses
                .iter()
                .all(|status| *status == TaskStatus::Succeeded)
            {
                task.status = TaskStatus::Ready;
            }
        }
    }

    fn running_count(&self) -> usize {
        self.tasks
            .values()
            .filter(|task| task.status == TaskStatus::Running)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn task(id: &str, dependencies: &[&str]) -> TaskSpec {
        TaskSpec {
            id: id.into(),
            parent_task_id: None,
            conversation_id: "conversation".into(),
            role: "worker".into(),
            description: id.into(),
            priority: 0,
            dependencies: dependencies.iter().map(|value| (*value).into()).collect(),
            capabilities: BTreeSet::new(),
            preferred_provider: None,
            assigned_provider: None,
            assigned_model: None,
            max_retries: 1,
        }
    }

    #[test]
    fn rejects_cycles() {
        let result = TaskGraph::new(vec![task("a", &["b"]), task("b", &["a"])]);
        assert!(matches!(result, Err(HarnessError::Validation(_))));
    }

    #[test]
    fn exposes_independent_work_in_parallel_then_dependency() {
        let graph = TaskGraph::new(vec![
            task("research", &[]),
            task("scan", &[]),
            task("synthesize", &["research", "scan"]),
        ])
        .unwrap();
        let mut scheduler = Scheduler::new(graph, 2).unwrap();
        let ready: Vec<String> = scheduler
            .ready()
            .iter()
            .map(|task| task.spec.id.clone())
            .collect();
        assert_eq!(ready, vec!["research", "scan"]);
        scheduler.start("research").unwrap();
        scheduler.start("scan").unwrap();
        scheduler.succeed("research").unwrap();
        assert!(scheduler.ready().is_empty());
        scheduler.succeed("scan").unwrap();
        assert_eq!(scheduler.ready()[0].spec.id, "synthesize");
    }

    #[test]
    fn retries_then_blocks_downstream() {
        let graph = TaskGraph::new(vec![task("work", &[]), task("review", &["work"])]).unwrap();
        let mut scheduler = Scheduler::new(graph, 1).unwrap();
        scheduler.start("work").unwrap();
        scheduler.fail("work").unwrap();
        assert_eq!(scheduler.ready()[0].retry_count, 1);
        scheduler.start("work").unwrap();
        scheduler.fail("work").unwrap();
        let snapshot = scheduler.snapshot();
        assert!(snapshot
            .tasks
            .iter()
            .any(|task| task.spec.id == "review" && task.status == TaskStatus::Blocked));
    }
}
