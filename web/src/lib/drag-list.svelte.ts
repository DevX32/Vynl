import { nearestIndex } from "@lib/reorder";

const DEFAULT_THRESHOLD = 4;

export interface DragGhost {
  x: number;
  y: number;
  label: string;
}

interface DragStartOptions {
  section?: string | null;
  idx: number;
  path: string;
  label: string;
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
    ghost: null as DragGhost | null,
    rowsEl: undefined as HTMLElement | undefined,
  });

  let grabPath: string | null = null;
  let ghostX = 0;
  let ghostLabel = "";
  let grabOffsetY = 0;
  let startY = 0;
  let pressed = false;
  let suppressClick = false;

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
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    drag.section = opts.section ?? null;
    drag.grabIdx = opts.idx;
    drag.overIdx = opts.idx;
    drag.dragging = false;
    drag.ghost = null;
    grabPath = opts.path;
    ghostX = rect.left;
    ghostLabel = opts.label;
    grabOffsetY = e.clientY - rect.top;
    startY = e.clientY;
    pressed = true;
    suppressClick = false;
  }

  function onMove(e: PointerEvent): void {
    if (!pressed) return;
    if (!drag.dragging) {
      if (Math.abs(e.clientY - startY) < threshold) return;
      drag.dragging = true;
    }
    drag.ghost = { x: ghostX, y: e.clientY - grabOffsetY, label: ghostLabel };
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
    drag.ghost = null;
    grabPath = null;
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

  function setupWindowListeners(): (() => void) | undefined {
    if (drag.grabIdx === null) return undefined;
    const onMoveEvent = (e: PointerEvent) => onMove(e);
    const onUp = () => finish();
    const onCancel = () => cancel();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") cancel();
    };
    window.addEventListener("pointermove", onMoveEvent);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("pointercancel", onCancel);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("pointermove", onMoveEvent);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("pointercancel", onCancel);
      window.removeEventListener("keydown", onKey);
    };
  }

  return { drag, start, cancel, shouldSkipClick, setupWindowListeners };
}
