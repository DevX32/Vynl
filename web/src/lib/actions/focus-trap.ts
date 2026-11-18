import type { Action } from "svelte/action";

const FOCUSABLE =
  'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])';

function visible(items: HTMLElement[]): HTMLElement[] {
  return items.filter((el) => {
    if (el.hasAttribute("disabled") || el.getAttribute("aria-disabled") === "true") {
      return false;
    }
    return el.offsetParent !== null || el === document.activeElement;
  });
}

export const focusTrap: Action<HTMLElement> = (node) => {
  const previouslyFocused = document.activeElement as HTMLElement | null;

  const onKeydown = (event: KeyboardEvent): void => {
    if (event.key !== "Tab") return;
    const items = visible(Array.from(node.querySelectorAll<HTMLElement>(FOCUSABLE)));
    if (items.length === 0) return;
    const first = items[0];
    const last = items[items.length - 1];
    const active = document.activeElement as HTMLElement | null;
    const inside = active ? node.contains(active) : false;
    if (event.shiftKey) {
      if (!inside || active === first) {
        event.preventDefault();
        event.stopPropagation();
        last.focus();
      }
    } else if (!inside || active === last) {
      event.preventDefault();
      event.stopPropagation();
      first.focus();
    }
  };

  const frame = requestAnimationFrame(() => {
    const first = visible(Array.from(node.querySelectorAll<HTMLElement>(FOCUSABLE)))[0];
    (first ?? node).focus?.();
  });

  node.addEventListener("keydown", onKeydown, { capture: true });
  return {
    destroy() {
      cancelAnimationFrame(frame);
      node.removeEventListener("keydown", onKeydown, { capture: true });
      previouslyFocused?.focus?.();
    },
  };
};
