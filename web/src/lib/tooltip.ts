const SHOW_DELAY = 320;
const GAP = 8;
const EDGE = 6;

const INSTALL_KEY = "__vynlTooltipInstalled";

type Placement = "top" | "bottom";

let tip: HTMLDivElement | null = null;
let active: HTMLElement | null = null;
let showTimer: number | null = null;
const suppressedTitles = new WeakMap<HTMLElement, string>();

function ensureTip(): HTMLDivElement {
  if (tip) return tip;
  tip = document.createElement("div");
  tip.className = "vynl-tooltip";
  tip.setAttribute("role", "tooltip");
  tip.setAttribute("aria-hidden", "true");
  tip.dataset.visible = "false";
  document.body.appendChild(tip);
  return tip;
}

function tooltipText(el: HTMLElement): string | null {
  const data = el.getAttribute("data-tooltip");
  if (data && data.trim()) return data.trim();
  const title = el.getAttribute("title");
  return title && title.trim() ? title.trim() : null;
}

function suppressNativeTitle(el: HTMLElement): void {
  const title = el.getAttribute("title");
  if (title === null) return;
  suppressedTitles.set(el, title);
  el.removeAttribute("title");
}

function restoreNativeTitle(el: HTMLElement): void {
  const title = suppressedTitles.get(el);
  if (title === undefined) return;
  suppressedTitles.delete(el);
  if (!el.hasAttribute("title")) el.setAttribute("title", title);
}

function clearShowTimer(): void {
  if (showTimer !== null) {
    window.clearTimeout(showTimer);
    showTimer = null;
  }
}

function place(el: HTMLElement): void {
  const node = ensureTip();
  const rect = el.getBoundingClientRect();
  const width = node.offsetWidth;
  const height = node.offsetHeight;

  let placement: Placement = "top";
  let left = rect.left + rect.width / 2;
  let top = rect.top - GAP;

  if (top - height < EDGE) {
    placement = "bottom";
    top = rect.bottom + GAP;
  }
  if (placement === "bottom" && top + height > window.innerHeight - EDGE) {
    placement = "top";
    top = rect.top - GAP;
  }

  const half = width / 2;
  left = Math.max(half + EDGE, Math.min(left, window.innerWidth - half - EDGE));

  const arrowX = Math.max(
    10,
    Math.min(width - 10, rect.left + rect.width / 2 - (left - half)),
  );

  node.dataset.placement = placement;
  node.style.left = `${Math.round(left)}px`;
  node.style.top = `${Math.round(top)}px`;
  node.style.setProperty("--tooltip-arrow-x", `${Math.round(arrowX)}px`);
}

function reveal(el: HTMLElement, text: string): void {
  showTimer = null;
  if (active !== el) return;
  const node = ensureTip();
  node.textContent = text;
  node.dataset.visible = "false";
  node.setAttribute("aria-hidden", "false");
  place(el);
  window.requestAnimationFrame(() => {
    if (active === el) node.dataset.visible = "true";
  });
}

function show(el: HTMLElement, immediate = false): void {
  if (active === el) return;
  hide();
  const text = tooltipText(el);
  if (!text) return;
  active = el;
  suppressNativeTitle(el);
  if (immediate) {
    reveal(el, text);
  } else {
    showTimer = window.setTimeout(() => reveal(el, text), SHOW_DELAY);
  }
}

function hide(): void {
  clearShowTimer();
  if (active) {
    restoreNativeTitle(active);
    active = null;
  }
  if (tip) {
    tip.dataset.visible = "false";
    tip.setAttribute("aria-hidden", "true");
  }
}

function findTarget(target: EventTarget | null): HTMLElement | null {
  if (!(target instanceof Element)) return null;
  return target.closest<HTMLElement>("[data-tooltip], [title]");
}

function onPointerOver(event: PointerEvent): void {
  if (event.pointerType === "touch") return;
  const el = findTarget(event.target);
  if (!el) return;
  if (active && (active === el || el.contains(event.target as Node))) return;
  show(el);
}

function onPointerOut(event: PointerEvent): void {
  if (!active) return;
  const related = event.relatedTarget;
  if (related instanceof Node && active.contains(related)) return;
  const target = event.target;
  if (target instanceof Node && (target === active || active.contains(target))) {
    hide();
  }
}

function onFocusIn(event: FocusEvent): void {
  const el = findTarget(event.target);
  if (!el) return;
  show(el, true);
}

function onFocusOut(event: FocusEvent): void {
  const el = findTarget(event.target);
  if (!el) return;
  const related = event.relatedTarget;
  if (related instanceof Node && el.contains(related)) return;
  if (active === el) hide();
}

export function installTooltips(): () => void {
  if (typeof window === "undefined" || typeof document === "undefined") {
    return () => {};
  }
  const marker = window as Window & { [INSTALL_KEY]?: boolean };
  if (marker[INSTALL_KEY]) return () => {};
  marker[INSTALL_KEY] = true;

  ensureTip();

  const onPointerDown = (): void => hide();
  const onScroll = (): void => hide();
  const onResize = (): void => hide();
  const onBlur = (): void => hide();
  const onKeyDown = (event: KeyboardEvent): void => {
    if (event.key === "Escape") hide();
  };

  document.addEventListener("pointerover", onPointerOver, true);
  document.addEventListener("pointerout", onPointerOut, true);
  document.addEventListener("focusin", onFocusIn, true);
  document.addEventListener("focusout", onFocusOut, true);
  document.addEventListener("pointerdown", onPointerDown, true);
  window.addEventListener("scroll", onScroll, true);
  window.addEventListener("resize", onResize);
  window.addEventListener("blur", onBlur);
  document.addEventListener("keydown", onKeyDown, true);

  return () => {
    document.removeEventListener("pointerover", onPointerOver, true);
    document.removeEventListener("pointerout", onPointerOut, true);
    document.removeEventListener("focusin", onFocusIn, true);
    document.removeEventListener("focusout", onFocusOut, true);
    document.removeEventListener("pointerdown", onPointerDown, true);
    window.removeEventListener("scroll", onScroll, true);
    window.removeEventListener("resize", onResize);
    window.removeEventListener("blur", onBlur);
    document.removeEventListener("keydown", onKeyDown, true);
    marker[INSTALL_KEY] = false;
  };
}
