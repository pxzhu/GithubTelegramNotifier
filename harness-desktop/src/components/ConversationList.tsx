import { useMemo, useState } from "react";

import { Icon } from "./Icon";
import type { ChatGroup, Conversation, Project } from "../types";

interface ConversationListProps {
  project?: Project;
  groups: ChatGroup[];
  conversations: Conversation[];
  activeConversationId: string;
  onSelectConversation: (conversationId: string) => void;
  onNewConversation: () => void;
  onToggleFavorite: (conversationId: string) => void;
  onOpenSearch: () => void;
}

export function ConversationList({
  project,
  groups,
  conversations,
  activeConversationId,
  onSelectConversation,
  onNewConversation,
  onToggleFavorite,
  onOpenSearch,
}: ConversationListProps) {
  const [collapsedGroups, setCollapsedGroups] = useState<Set<string>>(new Set());
  const [filter, setFilter] = useState("");

  const visible = useMemo(() => {
    const normalized = filter.trim().toLocaleLowerCase();
    if (!normalized) return conversations;
    return conversations.filter((conversation) =>
      `${conversation.title} ${conversation.preview}`.toLocaleLowerCase().includes(normalized),
    );
  }, [conversations, filter]);

  const toggleGroup = (groupId: string) => {
    setCollapsedGroups((current) => {
      const next = new Set(current);
      if (next.has(groupId)) next.delete(groupId);
      else next.add(groupId);
      return next;
    });
  };

  return (
    <aside className="conversation-list-pane" aria-label="Conversations">
      <header className="conversation-list-header">
        <div className="project-heading">
          <div>
            <h1>{project?.name ?? "Project"}</h1>
            <p>{project?.workspace}</p>
          </div>
          <button className="icon-button" aria-label="Project options"><Icon name="more" /></button>
        </div>

        <button className="new-chat-button" onClick={onNewConversation}>
          <Icon name="plus" size={16} />
          <span>New conversation</span>
          <kbd>⌘ N</kbd>
        </button>

        <label className="conversation-filter">
          <Icon name="search" size={15} />
          <input
            aria-label="Filter conversations"
            value={filter}
            onChange={(event) => setFilter(event.target.value)}
            placeholder="Filter conversations"
          />
          {filter && <button onClick={() => setFilter("")} aria-label="Clear filter"><Icon name="close" size={13} /></button>}
        </label>
      </header>

      <div className="conversation-scroll">
        {groups.map((group) => {
          const groupConversations = visible.filter((conversation) => conversation.groupId === group.id);
          if (filter && groupConversations.length === 0) return null;
          const collapsed = collapsedGroups.has(group.id);

          return (
            <section className="conversation-group" key={group.id}>
              <div className="group-heading">
                <button onClick={() => toggleGroup(group.id)} aria-expanded={!collapsed}>
                  <Icon className={collapsed ? "" : "rotated"} name="arrow" size={13} />
                  <span>{group.name}</span>
                  <small>{groupConversations.length}</small>
                </button>
                <button className="icon-button subtle group-more" aria-label={`${group.name} options`}><Icon name="more" size={15} /></button>
              </div>
              {!collapsed && (
                <div className="group-items">
                  {groupConversations.map((conversation) => (
                    <button
                      className={`conversation-item ${conversation.id === activeConversationId ? "active" : ""}`}
                      key={conversation.id}
                      onClick={() => onSelectConversation(conversation.id)}
                    >
                      <span className="conversation-item-top">
                        <span className="conversation-title">{conversation.title}</span>
                        <time>{conversation.updatedAt}</time>
                      </span>
                      <span className="conversation-preview">{conversation.preview}</span>
                      <span className="conversation-flags">
                        {conversation.status === "working" && <span className="working-pill"><span /> Working</span>}
                        {conversation.status === "attention" && <span className="attention-pill">Needs attention</span>}
                        {conversation.tags.slice(0, 2).map((tag) => <span className="mini-tag" key={tag}>{tag}</span>)}
                        {conversation.favorite && (
                          <span
                            className="favorite-star"
                            role="button"
                            aria-label="Remove from favorites"
                            onClick={(event) => { event.stopPropagation(); onToggleFavorite(conversation.id); }}
                          >★</span>
                        )}
                        {conversation.unread && <span className="unread-dot" aria-label="Unread" />}
                      </span>
                    </button>
                  ))}
                </div>
              )}
            </section>
          );
        })}

        {visible.length === 0 && (
          <div className="empty-filter">
            <Icon name="search" />
            <strong>No conversations found</strong>
            <button onClick={onOpenSearch}>Search all projects</button>
          </div>
        )}
      </div>
    </aside>
  );
}
