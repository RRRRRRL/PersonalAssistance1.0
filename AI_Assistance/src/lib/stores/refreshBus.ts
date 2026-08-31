type RefreshTopic = "finance" | "alerts" | "orchestration" | "calendar" | "music";

type RefreshListener = () => void;

const listeners: Record<RefreshTopic, Set<RefreshListener>> = {
  finance: new Set(),
  alerts: new Set(),
  orchestration: new Set(),
  calendar: new Set(),
  music: new Set(),
};

export function emitRefresh(topic: RefreshTopic): void {
  for (const listener of listeners[topic]) {
    listener();
  }
}

export function onRefresh(
  topic: RefreshTopic,
  listener: RefreshListener,
): () => void {
  listeners[topic].add(listener);
  return () => {
    listeners[topic].delete(listener);
  };
}
