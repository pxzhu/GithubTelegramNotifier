import { FormEvent, useEffect, useRef, useState } from "react";

import { Icon } from "./Icon";

interface CreateProjectDialogProps {
  open: boolean;
  busy: boolean;
  onClose: () => void;
  onChooseWorkspace: () => Promise<string | null>;
  onCreate: (name: string, workspace: string) => Promise<void>;
}

export function CreateProjectDialog({
  open,
  busy,
  onClose,
  onChooseWorkspace,
  onCreate,
}: CreateProjectDialogProps) {
  const [name, setName] = useState("");
  const [workspace, setWorkspace] = useState("");
  const nameRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!open) return;
    setName("");
    setWorkspace("");
    window.setTimeout(() => nameRef.current?.focus(), 0);
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !busy) onClose();
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [busy, onClose, open]);

  if (!open) return null;

  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (!name.trim() || busy) return;
    void onCreate(name.trim(), workspace.trim());
  };

  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={(event) => {
      if (event.currentTarget === event.target && !busy) onClose();
    }}>
      <form className="project-dialog" role="dialog" aria-modal="true" aria-labelledby="new-project-title" onSubmit={submit}>
        <header>
          <span className="dialog-icon"><Icon name="folder" size={19} /></span>
          <div><h2 id="new-project-title">Create a project</h2><p>Chats, agent runs, and usage stay separated by project.</p></div>
          <button type="button" className="icon-button" onClick={onClose} disabled={busy} aria-label="Close"><Icon name="close" /></button>
        </header>
        <div className="dialog-fields">
          <div className="dialog-field"><label htmlFor="project-name">Project name</label><input id="project-name" ref={nameRef} value={name} onChange={(event) => setName(event.target.value)} maxLength={120} placeholder="AI Harness" /></div>
          <div className="dialog-field"><label htmlFor="project-workspace">Workspace <small>Optional</small></label><div className="workspace-field"><input id="project-workspace" value={workspace} onChange={(event) => setWorkspace(event.target.value)} placeholder="/path/to/repository" /><button type="button" className="secondary-button" aria-label="Choose workspace folder" onClick={() => { void onChooseWorkspace().then((path) => { if (path) setWorkspace(path); }); }}>Choose</button></div></div>
        </div>
        <footer><button type="button" className="secondary-button" onClick={onClose} disabled={busy}>Cancel</button><button type="submit" className="primary-button" disabled={!name.trim() || busy}>{busy ? <span className="tiny-spinner" /> : <Icon name="plus" size={14} />} Create project</button></footer>
      </form>
    </div>
  );
}
