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

  let centers: number[] | null = null;
  let bandTop = Number.NEGATIVE_INFINITY;
  let bandBottom = Number.POSITIVE_INFINITY;

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

  function measureCenters(y: number): number | null {
    const measured = rowCenters();
    centers = measured;
    const idx = nearestIndex(measured, y);
    if (idx === null) {
      bandTop = Number.NEGATIVE_INFINITY;
      bandBottom = Number.POSITIVE_INFINITY;
    } else {
      bandTop =
        idx > 0
          ? (measured[idx - 1] + measured[idx]) / 2
          : Number.NEGATIVE_INFINITY;
      bandBottom =
        idx < measured.length - 1
          ? (measured[idx] + measured[idx + 1]) / 2
          : Number.POSITIVE_INFINITY;
    }
    return idx;
  }

  function overIndex(y: number): number | null {
    if (!centers || y <= bandTop || y > bandBottom) return measureCenters(y);
    return nearestIndex(centers, y);
  }

  function invalidateCenters(): void {
    centers = null;
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
    invalidateCenters();
    attach();
  }

  function onMove(e: PointerEvent): void {
    if (!pressed) return;
    if (!drag.dragging) {
      if (Math.abs(e.clientY - startY) < threshold) return;
      drag.dragging = true;
    }
    const next = overIndex(e.clientY);
    if (next !== drag.overIdx) {
      invalidateCenters();
    }
    drag.overIdx = next;
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
    invalidateCenters();
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
    const onScrollEvent = () => invalidateCenters();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") cancel();
    };
    window.addEventListener("pointermove", onMoveEvent);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("pointercancel", onCancelEvent);
    window.addEventListener("scroll", onScrollEvent, true);
    window.addEventListener("keydown", onKey);
    teardown = () => {
      window.removeEventListener("pointermove", onMoveEvent);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("pointercancel", onCancelEvent);
      window.removeEventListener("scroll", onScrollEvent, true);
      window.removeEventListener("keydown", onKey);
      teardown = null;
    };
  }

  return { drag, start, cancel, shouldSkipClick };
}
