import type { SVGProps } from "react";

export type IconName =
  | "activity"
  | "archive"
  | "arrow"
  | "attach"
  | "bolt"
  | "branch"
  | "check"
  | "chevron"
  | "close"
  | "code"
  | "command"
  | "download"
  | "file"
  | "folder"
  | "gear"
  | "globe"
  | "grid"
  | "info"
  | "layers"
  | "menu"
  | "message"
  | "more"
  | "plus"
  | "refresh"
  | "search"
  | "send"
  | "sidebar"
  | "sparkles"
  | "stop"
  | "terminal"
  | "user"
  | "warning";

interface IconProps extends SVGProps<SVGSVGElement> {
  name: IconName;
  size?: number;
}

const paths: Record<IconName, React.ReactNode> = {
  activity: <><path d="M4 12h3l2-6 4 12 2-6h5" /></>,
  archive: <><path d="M4 7h16v13H4z" /><path d="M3 4h18v3H3zM9 11h6" /></>,
  arrow: <><path d="m8 5 7 7-7 7" /></>,
  attach: <><path d="m9.5 12.5 5.7-5.7a3 3 0 1 1 4.2 4.2l-8.5 8.5a5 5 0 0 1-7.1-7.1l8.5-8.5" /><path d="m7.4 14.6 7.1-7.1" /></>,
  bolt: <><path d="m13 2-8 12h7l-1 8 8-12h-7z" /></>,
  branch: <><circle cx="6" cy="5" r="2" /><circle cx="18" cy="6" r="2" /><circle cx="6" cy="19" r="2" /><path d="M6 7v10M8 11c6 0 4-5 8-5" /></>,
  check: <><path d="m5 12 4 4L19 6" /></>,
  chevron: <><path d="m8 10 4 4 4-4" /></>,
  close: <><path d="m6 6 12 12M18 6 6 18" /></>,
  code: <><path d="m8 9-4 3 4 3M16 9l4 3-4 3M14 5l-4 14" /></>,
  command: <><path d="M9 6V5a3 3 0 1 0-3 3h12a3 3 0 1 0-3-3v14a3 3 0 1 0 3-3H6a3 3 0 1 0 3 3z" /></>,
  download: <><path d="M12 3v12m0 0 5-5m-5 5-5-5M4 20h16" /></>,
  file: <><path d="M6 2h8l4 4v16H6z" /><path d="M14 2v5h5" /></>,
  folder: <><path d="M3 6h7l2 2h9v11H3z" /></>,
  gear: <><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.9l.1.1-2.8 2.8-.1-.1a1.7 1.7 0 0 0-1.9-.3 1.7 1.7 0 0 0-1 1.6v.2h-4V21a1.7 1.7 0 0 0-1-1.6 1.7 1.7 0 0 0-1.9.3l-.1.1L4.2 17l.1-.1a1.7 1.7 0 0 0 .3-1.9A1.7 1.7 0 0 0 3 14H2.8v-4H3a1.7 1.7 0 0 0 1.6-1 1.7 1.7 0 0 0-.3-1.9L4.2 7 7 4.2l.1.1A1.7 1.7 0 0 0 9 4.6 1.7 1.7 0 0 0 10 3v-.2h4V3a1.7 1.7 0 0 0 1 1.6 1.7 1.7 0 0 0 1.9-.3l.1-.1L19.8 7l-.1.1a1.7 1.7 0 0 0-.3 1.9 1.7 1.7 0 0 0 1.6 1h.2v4H21a1.7 1.7 0 0 0-1.6 1Z" /></>,
  globe: <><circle cx="12" cy="12" r="9" /><path d="M3 12h18M12 3c3 3 3 15 0 18M12 3c-3 3-3 15 0 18" /></>,
  grid: <><rect x="3" y="3" width="7" height="7" rx="1" /><rect x="14" y="3" width="7" height="7" rx="1" /><rect x="3" y="14" width="7" height="7" rx="1" /><rect x="14" y="14" width="7" height="7" rx="1" /></>,
  info: <><circle cx="12" cy="12" r="9" /><path d="M12 11v6M12 7h.01" /></>,
  layers: <><path d="m12 3 9 5-9 5-9-5z" /><path d="m3 12 9 5 9-5M3 16l9 5 9-5" /></>,
  menu: <><path d="M4 7h16M4 12h16M4 17h16" /></>,
  message: <><path d="M4 4h16v13H8l-4 4z" /></>,
  more: <><circle cx="5" cy="12" r="1" fill="currentColor" stroke="none" /><circle cx="12" cy="12" r="1" fill="currentColor" stroke="none" /><circle cx="19" cy="12" r="1" fill="currentColor" stroke="none" /></>,
  plus: <><path d="M12 5v14M5 12h14" /></>,
  refresh: <><path d="M20 6v5h-5M4 18v-5h5" /><path d="M18.5 9A7 7 0 0 0 6.2 6.2L4 8m2 7a7 7 0 0 0 11.8 2.8L20 16" /></>,
  search: <><circle cx="10.5" cy="10.5" r="6.5" /><path d="m16 16 5 5" /></>,
  send: <><path d="m3 3 18 9-18 9 4-9zM7 12h14" /></>,
  sidebar: <><rect x="3" y="3" width="18" height="18" rx="2" /><path d="M9 3v18" /></>,
  sparkles: <><path d="m12 3 1.2 3.8L17 8l-3.8 1.2L12 13l-1.2-3.8L7 8l3.8-1.2zM6 14l.8 2.2L9 17l-2.2.8L6 20l-.8-2.2L3 17l2.2-.8zM18 14l.7 1.3L20 16l-1.3.7L18 18l-.7-1.3L16 16l1.3-.7z" /></>,
  stop: <><rect x="6" y="6" width="12" height="12" rx="2" /></>,
  terminal: <><rect x="3" y="4" width="18" height="16" rx="2" /><path d="m7 9 3 3-3 3M13 16h4" /></>,
  user: <><circle cx="12" cy="8" r="4" /><path d="M4 21c.6-4.3 3.3-7 8-7s7.4 2.7 8 7" /></>,
  warning: <><path d="M12 3 2.8 20h18.4z" /><path d="M12 9v5M12 17h.01" /></>,
};

export function Icon({ name, size = 18, ...props }: IconProps) {
  return (
    <svg
      aria-hidden="true"
      fill="none"
      height={size}
      viewBox="0 0 24 24"
      width={size}
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth="1.7"
      {...props}
    >
      {paths[name]}
    </svg>
  );
}
