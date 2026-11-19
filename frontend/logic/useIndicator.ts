import {
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
  type RefObject,
} from "react";

/**
 * Position of a sliding highlight under the selected item of a row of
 * options (tabs, segmented controls). The container marks the selected
 * child with `data-selected`; the highlight then moves there with a CSS
 * transition instead of jumping. Re-measures after every render (cheap:
 * two numbers) and when the row resizes, so it never points at a stale
 * spot after the items change.
 */
export function useIndicator(
  container: RefObject<HTMLElement | null>,
  selectedKey: string,
): CSSProperties {
  const [style, setStyle] = useState<CSSProperties>({ opacity: 0 });
  // Last measured "left,width", so an unchanged spot doesn't re-render.
  const last = useRef("");

  useLayoutEffect(() => {
    const row = container.current;
    if (!row) return;
    const measure = () => {
      const el = row.querySelector<HTMLElement>("[data-selected='true']");
      const spot = el ? `${el.offsetLeft},${el.offsetWidth}` : "none";
      if (spot === last.current) return;
      last.current = spot;
      setStyle(
        el
          ? {
              width: el.offsetWidth,
              transform: `translateX(${el.offsetLeft}px)`,
              opacity: 1,
            }
          : { opacity: 0 },
      );
    };
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(row);
    return () => ro.disconnect();
  });

  // `selectedKey` changes the marked child; the effect above re-measures
  // on that render like any other.
  void selectedKey;
  return style;
}
