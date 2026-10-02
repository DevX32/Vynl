import { nearestIndex } from "@lib/reorder";

const DEFAULT_THRESHOLD = 4;

interface DragStartOptions {
  section?: string | null;
  idx: number;
  path: string;
}

interface DragListOptions {
  pathAt: (section: string | null, idx: number) => string | undefined;
  onReorder: (fromPath: string, toPath: string) => void;
  threshold?: number;
  rowSelector?: (section: string | null) => string;
}

function defaultRowSelector(section: string | null): string {
  return section ? `.row.${section}-row` : ".row";
}

export function useDragList(options: DragListOptions) {
  const threshold = options.threshold ?? DEFAULT_THRESHOLD;
  const rowSelector = options.rowSelector ?? defaultRowSelector;

  const drag = $state({
    section: null as string | null,
    grabIdx: null as number | null,
    overIdx: null as number | null,
    dragging: false,
    rowsEl: undefined as HTMLElement | undefined,
  });

  let grabPath: string | null = null;
  let startY = 0;
  let pressed = false;
  let suppressClick = false;
  let teardown: (() => void) | null = null;

  function rowCenters(): number[] {
    if (!drag.rowsEl) return [];
    const rows = drag.rowsEl.querySelectorAll<HTMLElement>(
      rowSelector(drag.section),
    );
    return Array.from(rows, (row) => {
      const rect = row.getBoundingClientRect();
      return rect.top + rect.height / 2;
    });
  }

  function start(e: PointerEvent, opts: DragStartOptions): void {
    if (e.button !== 0) return;
    drag.section = opts.section ?? null;
    drag.grabIdx = opts.idx;
    drag.overIdx = opts.idx;
    drag.dragging = false;
    grabPath = opts.path;
    startY = e.clientY;
    pressed = true;
    suppressClick = false;
    attach();
  }

  function onMove(e: PointerEvent): void {
    if (!pressed) return;
    if (!drag.dragging) {
      if (Math.abs(e.clientY - startY) < threshold) return;
      drag.dragging = true;
    }
    drag.overIdx = nearestIndex(rowCenters(), e.clientY);
  }

  function finish(): void {
    if (drag.dragging) {
      suppressClick = true;
      const from = grabPath;
      const to =
        drag.overIdx !== null
          ? options.pathAt(drag.section, drag.overIdx)
          : undefined;
      if (from && to && to !== from) {
        options.onReorder(from, to);
      }
    }
    reset();
  }

  function reset(): void {
    pressed = false;
    drag.section = null;
    drag.grabIdx = null;
    drag.overIdx = null;
    drag.dragging = false;
    grabPath = null;
    detach();
  }

  function cancel(): void {
    if (drag.dragging) suppressClick = true;
    reset();
  }

  function shouldSkipClick(): boolean {
    if (!suppressClick) return false;
    suppressClick = false;
    return true;
  }

  function detach(): void {
    if (!teardown) return;
    teardown();
  }

  function attach(): void {
    if (teardown) return;
    const onMoveEvent = (e: PointerEvent) => onMove(e);
    const onUp = () => finish();
    const onCancelEvent = () => cancel();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") cancel();
    };
    window.addEventListener("pointermove", onMoveEvent);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("pointercancel", onCancelEvent);
    window.addEventListener("keydown", onKey);
    teardown = () => {
      window.removeEventListener("pointermove", onMoveEvent);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("pointercancel", onCancelEvent);
      window.removeEventListener("keydown", onKey);
      teardown = null;
    };
  }

  return { drag, start, cancel, shouldSkipClick };
}
