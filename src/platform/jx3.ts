import { invoke } from "@tauri-apps/api/core";

export type NewsKind = "announcement" | "skillChange";

export interface NewsItem {
  id: string;
  title: string;
  publishedAt: string;
  sourceUrl: string;
  isSkillChange: boolean;
}

export interface NewsPage {
  kind: NewsKind;
  page: number;
  pageSize: number;
  totalItems?: number;
  totalPages?: number;
  hasMore: boolean;
  fetchedAt: number;
  stale: boolean;
  items: NewsItem[];
}

export interface ServerOption {
  name: string;
  zone: string;
  monitored: boolean;
}

export type ServerAvailability = "unknown" | "closed" | "open";

export interface ServerStatus {
  name: string;
  zone: string;
  status: ServerAvailability;
  checkedAt: number;
}

export function getCachedNewsPage(kind: NewsKind, page: number): Promise<NewsPage | null> {
  return invoke("get_cached_jx3_news_page", { kind, page });
}

export function fetchNewsPage(
  kind: NewsKind,
  page: number,
  force = false,
): Promise<NewsPage> {
  return invoke("fetch_jx3_news_page", { kind, page, force });
}

export function getServerOptions(): Promise<ServerOption[]> {
  return invoke("get_jx3_server_options");
}

export function checkServer(serverName: string): Promise<ServerStatus> {
  return invoke("check_jx3_server", { serverName });
}

export function setServerMonitoring(
  serverName: string,
  enabled: boolean,
): Promise<string[]> {
  return invoke("set_jx3_server_monitoring", { serverName, enabled });
}

export function stopAllServerMonitoring(): Promise<void> {
  return invoke("stop_all_jx3_server_monitoring");
}

export function openOfficialNews(url: string): Promise<void> {
  return invoke("open_jx3_official_url", { url });
}
