import type {
  AppSettings,
  MetricLine,
  ProviderSnapshotState,
} from "@ai-usage-dashboard/core";
import { Pin, RefreshCw, Settings as SettingsIcon } from "lucide-react";
import type { TFunction } from "../i18n";
import { formatRelative } from "../lib/format";
import { providerLogo } from "../lib/provider-logos";
import { toCardState } from "../lib/card-state";
import { useUiStore } from "../stores/ui-store";

function preferredProgress(state: ProviderSnapshotState) {
  const progress =
    state.snapshot?.lines.filter(
      (line): line is Extract<MetricLine, { type: "progress" }> =>
        line.type === "progress",
    ) ?? [];

  const fiveHour = progress.find((line) =>
    /session|current.session|5[ -]?h(?:our)?/i.test(line.label),
  );
  const oneWeek = progress.find((line) => /week|7[ -]?day/i.test(line.label));

  return fiveHour ?? oneWeek ?? progress[0];
}

function periodLabel(line: Extract<MetricLine, { type: "progress" }>) {
  return /session|current.session|5[ -]?h(?:our)?/i.test(line.label)
    ? "5h"
    : "1w";
}

function fallbackValue(state: ProviderSnapshotState) {
  if (state.provider.id === "claude") return "재로그인 필요";
  if (state.provider.id === "grok") return "로그인 필요";

  const quota = state.snapshot?.lines.find(
    (line): line is Extract<MetricLine, { type: "badge" }> =>
      line.type === "badge" && line.label === "Quota",
  );
  if (quota) return quota.value === "Console only" ? "콘솔 확인" : quota.value;

  const usefulText = state.snapshot?.lines.find(
    (line): line is Extract<MetricLine, { type: "text" }> =>
      line.type === "text" && !/token/i.test(line.value),
  );
  return usefulText?.value ?? "데이터 없음";
}

function islandName(state: ProviderSnapshotState) {
  if (state.provider.id === "alibaba-token-plan") return "Alibaba";
  if (state.provider.id === "opencode-go") return "OpenCode Go";
  return state.provider.displayName;
}

function ProviderIslandTile({
  state,
  settings,
  t,
}: {
  state: ProviderSnapshotState;
  settings: AppSettings;
  t: TFunction;
}) {
  const setActive = useUiStore((store) => store.setActiveView);
  const progress = preferredProgress(state);
  const cardState = toCardState(state, settings);
  const percent = progress
    ? Math.round(Math.max(0, Math.min(1, progress.used / progress.limit)) * 100)
    : null;
  const unavailable =
    cardState.kind === "unconfigured" || cardState.kind === "error";
  const status = unavailable
    ? state.provider.id === "claude"
      ? "재로그인 필요"
      : "연결 필요"
    : fallbackValue(state);
  const compact = settings.compactMode;

  return (
    <button
      type="button"
      onClick={() => setActive(state.provider.id)}
      className={`group flex min-w-0 flex-1 flex-col justify-center border-l text-left transition-colors first:border-l-0 hover:bg-white/5 ${
        compact
          ? "gap-1 border-white/5 px-2.5 py-1.5"
          : "gap-1.5 border-white/8 px-3.5 py-2"
      }`}
      aria-label={state.provider.displayName}
    >
      <div className="flex items-center gap-1.5">
        <span
          aria-hidden="true"
          className={`${compact ? "h-3 w-3" : "h-3.5 w-3.5"} shrink-0`}
          style={{
            backgroundColor: state.provider.brandColor,
            WebkitMaskImage: `url("${providerLogo[state.provider.id]}")`,
            WebkitMaskPosition: "center",
            WebkitMaskRepeat: "no-repeat",
            WebkitMaskSize: "contain",
          }}
        />
        <span
          className={`truncate font-medium text-white/60 ${compact ? "text-[9px]" : "text-[10px]"}`}
        >
          {islandName(state)}
        </span>
        {progress && !unavailable ? (
          <span
            className={`ml-auto rounded-full bg-white/7 font-semibold text-white/40 ${compact ? "px-1 py-px text-[7px]" : "px-1.5 py-0.5 text-[8px]"}`}
          >
            {periodLabel(progress)}
          </span>
        ) : null}
      </div>

      {progress && !unavailable ? (
        <>
          <div className="flex items-end justify-between gap-2">
            <span
              className={`${compact ? "text-[16px]" : "text-[18px]"} font-semibold leading-none tracking-tight text-white tabular-nums`}
            >
              {percent}%
            </span>
            <span
              className={`truncate text-white/35 ${compact ? "text-[8px]" : "text-[9px]"}`}
              title={
                progress.resetsAt
                  ? new Date(progress.resetsAt).toLocaleString("ko-KR")
                  : undefined
              }
            >
              {progress.resetsAt
                ? compact
                  ? formatRelative(progress.resetsAt, t)
                  : `${formatRelative(progress.resetsAt, t)} 재설정`
                : compact
                  ? "—"
                  : "재설정 정보 없음"}
            </span>
          </div>
          <div
            className={`${compact ? "h-px" : "h-0.5"} overflow-hidden rounded-full bg-white/10`}
          >
            <div
              className="h-full rounded-full transition-[width]"
              style={{
                width: `${percent}%`,
                backgroundColor: state.provider.brandColor,
              }}
            />
          </div>
        </>
      ) : (
        <div className={`flex items-end ${compact ? "h-[19px]" : "h-[23px]"}`}>
          <span
            className={`truncate font-medium text-white/60 ${compact ? "text-[9px]" : "text-[11px]"}`}
          >
            {status}
          </span>
        </div>
      )}
    </button>
  );
}

export function IslandOverview({
  states,
  settings,
  refreshing,
  onRefresh,
  onAlwaysVisibleChange,
  t,
}: {
  states: ProviderSnapshotState[];
  settings: AppSettings;
  refreshing: boolean;
  onRefresh: () => void;
  onAlwaysVisibleChange: (value: boolean) => void;
  t: TFunction;
}) {
  const setActive = useUiStore((store) => store.setActiveView);

  return (
    <section
      className={`h-screen w-screen select-none bg-transparent ${settings.compactMode ? "p-1.5" : "p-2"}`}
    >
      <div
        className={`flex h-full overflow-hidden border bg-[#050505]/95 backdrop-blur-2xl ${
          settings.compactMode
            ? "rounded-[23px] border-white/10 shadow-[0_10px_34px_rgba(0,0,0,0.48)]"
            : "rounded-[27px] border-white/12 shadow-[0_14px_45px_rgba(0,0,0,0.55)]"
        }`}
      >
        <button
          type="button"
          onClick={onRefresh}
          className={`flex shrink-0 items-center justify-center text-white/40 transition-colors hover:bg-white/5 hover:text-white ${settings.compactMode ? "w-9" : "w-10"}`}
          aria-label={t("footer.refreshAction")}
        >
          <RefreshCw
            className={`${settings.compactMode ? "h-3 w-3" : "h-3.5 w-3.5"} ${refreshing ? "animate-spin" : ""}`}
          />
        </button>

        <div className="flex min-w-0 flex-1">
          {states.map((state) => (
            <ProviderIslandTile
              key={state.provider.id}
              state={state}
              settings={settings}
              t={t}
            />
          ))}
        </div>

        <button
          type="button"
          onClick={() => onAlwaysVisibleChange(!settings.alwaysVisible)}
          className={`flex shrink-0 items-center justify-center border-l border-white/5 transition-colors hover:bg-white/5 hover:text-white ${
            settings.compactMode ? "w-8" : "w-9"
          } ${settings.alwaysVisible ? "bg-white/8 text-white" : "text-white/35"}`}
          aria-label={t("settings.display.alwaysVisibleToggle")}
          aria-pressed={settings.alwaysVisible}
          title={t("settings.display.alwaysVisible")}
        >
          <Pin
            className={`${settings.compactMode ? "h-3 w-3" : "h-3.5 w-3.5"} ${settings.alwaysVisible ? "fill-current" : ""}`}
          />
        </button>

        <button
          type="button"
          onClick={() => setActive("settings")}
          className={`flex shrink-0 items-center justify-center border-l border-white/5 text-white/40 transition-colors hover:bg-white/5 hover:text-white ${settings.compactMode ? "w-9" : "w-10"}`}
          aria-label={t("nav.settings")}
        >
          <SettingsIcon
            className={settings.compactMode ? "h-3 w-3" : "h-3.5 w-3.5"}
          />
        </button>
      </div>
    </section>
  );
}
