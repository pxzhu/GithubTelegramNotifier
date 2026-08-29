import { useEffect, useRef, useState } from "react";

import { Icon } from "./Icon";
import type { SearchResult } from "../types";

interface SearchScreenProps {
  onSearch: (query: string) => Promise<SearchResult[]>;
  onOpenConversation: (conversationId: string) => void;
}

export function SearchScreen({ onSearch, onOpenConversation }: SearchScreenProps) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [recentSearches, setRecentSearches] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => { inputRef.current?.focus(); }, []);

  useEffect(() => {
    let current = true;
    if (!query.trim()) { setResults([]); setLoading(false); return; }
    setLoading(true);
    const timer = window.setTimeout(() => {
      onSearch(query).then((next) => {
        if (current) {
          setResults(next);
          setLoading(false);
          setRecentSearches((previous) => [query.trim(), ...previous.filter((item) => item !== query.trim())].slice(0, 6));
        }
      }).catch(() => {
        if (current) {
          setResults([]);
          setLoading(false);
        }
      });
    }, 180);
    return () => { current = false; window.clearTimeout(timer); };
  }, [onSearch, query]);

  const selectResult = (result: SearchResult) => {
    if (result.conversationId) onOpenConversation(result.conversationId);
  };

  return (
    <main className="screen-page search-screen">
      <div className="page-toolbar"><div><span className="page-eyebrow">Local full-text index</span><h1>Search</h1></div><span className="index-state"><span /> Up to date</span></div>
      <div className="global-search-box">
        <Icon name="search" size={21} />
        <input ref={inputRef} value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Search conversations, messages, files, tags…" aria-label="Search everything" />
        {loading && <span className="tiny-spinner dark" />}
        {query && !loading && <button className="icon-button" onClick={() => setQuery("")} aria-label="Clear search"><Icon name="close" size={15} /></button>}
        <kbd>⌘ ⇧ F</kbd>
      </div>
      <div className="search-filters"><button className="active">Everything</button><button>Conversations</button><button>Messages</button><button>Agent results</button><button>Files</button><span /><button><Icon name="folder" size={13} /> All projects <Icon name="chevron" size={11} /></button></div>

      {!query && (
        <div className="search-idle">
          <section><header><h2>Recent searches</h2>{recentSearches.length > 0 && <button onClick={() => setRecentSearches([])}>Clear</button>}</header><div className="recent-search-list">{recentSearches.length === 0 ? <p className="empty-recent-searches">Search history for this window will appear here.</p> : recentSearches.map((search) => <button key={search} onClick={() => setQuery(search)}><Icon name="refresh" size={14} /><span>{search}</span><Icon name="arrow" size={13} /></button>)}</div></section>
          <section className="search-tip"><span className="search-tip-icon"><Icon name="sparkles" /></span><div><h3>Everything stays on this device</h3><p>Search uses the local SQLite FTS index. Your query and results are never sent to a provider.</p></div></section>
        </div>
      )}

      {query && !loading && (
        <div className="search-results-page">
          <div className="result-count">{results.length} results <span>for “{query}”</span></div>
          {results.map((result) => (
            <button className="search-result-card" key={result.id} onClick={() => selectResult(result)}>
              <span className={`search-result-icon ${result.type}`}><Icon name={result.type === "conversation" ? "message" : result.type === "file" ? "file" : "search"} size={16} /></span>
              <span className="search-result-copy"><span><strong>{result.title}</strong><em>{result.type}</em></span><p>{result.excerpt}</p><small>{result.project ?? "AI Harness"}</small></span>
              <Icon name="arrow" size={15} />
            </button>
          ))}
          {results.length === 0 && <div className="no-search-results"><Icon name="search" size={28} /><h3>No local matches</h3><p>Try another phrase, filename or tag.</p></div>}
        </div>
      )}
    </main>
  );
}
