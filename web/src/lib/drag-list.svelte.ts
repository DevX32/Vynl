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

interface RowBox {
  index: number;
  center: number;
  height: number;
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
  let boxes: RowBox[] | null = null;
  let bandTop = Number.NEGATIVE_INFINITY;
  let bandBottom = Number.POSITIVE_INFINITY;

  function rowBoxes(): RowBox[] {
    if (!drag.rowsEl) return [];
    const rows = drag.rowsEl.querySelectorAll<HTMLElement>(
      rowSelector(drag.section),
    );
    return Array.from(rows, (row) => {
      const rect = row.getBoundingClientRect();
      const source = row.dataset.index
        ? row
        : (row.closest<HTMLElement>("[data-index]") ?? row);
      const attr = Number(source.dataset.index);
      return {
        index: Number.isFinite(attr) ? attr : -1,
        center: rect.top + rect.height / 2,
        height: rect.height,
      };
    });
  }

  function nearestBox(candidates: RowBox[], y: number): number {
    let best = 0;
    let bestDist = Infinity;
    for (let i = 0; i < candidates.length; i += 1) {
      const dist = Math.abs(y - candidates[i].center);
      if (dist < bestDist) {
        bestDist = dist;
        best = i;
      }
    }
    return best;
  }

  function measureCenters(y: number): number | null {
    const measured = rowBoxes();
    centers = measured.map((b) => b.center);
    boxes = measured;
    if (measured.length === 0) {
      bandTop = Number.NEGATIVE_INFINITY;
      bandBottom = Number.POSITIVE_INFINITY;
      return null;
    }

    const pos = nearestBox(measured, y);
    const boxed = measured[pos];
    const base = boxed.index >= 0 ? boxed.index : pos;

    const first = boxes[0];
    const last = boxes[boxes.length - 1];
    const height = boxed.height || 0;
    let index = base;

    if (height > 0 && y < first.center - first.height / 2) {
      const steps = Math.ceil((first.center - first.height / 2 - y) / height);
      index = base - steps;
    } else if (height > 0 && y > last.center + last.height / 2) {
      const steps = Math.floor((y - (last.center + last.height / 2)) / height);
      index = base + steps;
    }

    if (index !== base) {
      bandTop = Number.NEGATIVE_INFINITY;
      bandBottom = Number.POSITIVE_INFINITY;
      return index;
    }

    bandTop =
      pos > 0
        ? (measured[pos - 1].center + measured[pos].center) / 2
        : Number.NEGATIVE_INFINITY;
    bandBottom =
      pos < measured.length - 1
        ? (measured[pos].center + measured[pos + 1].center) / 2
        : Number.POSITIVE_INFINITY;
    return index;
  }

  function overIndex(y: number): number | null {
    if (!centers || !boxes || y <= bandTop || y > bandBottom) {
      return measureCenters(y);
    }
    const pos = nearestIndex(centers, y);
    if (pos === null) return null;
    const boxed = boxes[pos];
    return boxed.index >= 0 ? boxed.index : pos;
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
