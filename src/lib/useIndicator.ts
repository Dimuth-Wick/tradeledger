import {
  useLayoutEffect,
  useState,
  type CSSProperties,
  type RefObject,
} from "react";

/**
 * Position of a sliding highlight under the selected item of a row of
 * options (tabs, segmented controls). The container marks the selected
 * child with `data-selected`; the highlight then moves there with a CSS
 * transition instead of jumping. Re-measures when the row resizes (window
 * width, font load).
 */
export function useIndicator(
  container: RefObject<HTMLElement | null>,
  selectedKey: string,
): CSSProperties {
  const [style, setStyle] = useState<CSSProperties>({ opacity: 0 });

  useLayoutEffect(() => {
    const row = container.current;
    if (!row) return;
    const measure = () => {
      const el = row.querySelector<HTMLElement>("[data-selected='true']");
      if (!el) {
        setStyle({ opacity: 0 });
        return;
      }
      setStyle({
        width: el.offsetWidth,
        transform: `translateX(${el.offsetLeft}px)`,
        opacity: 1,
      });
    };
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(row);
    return () => ro.disconnect();
  }, [container, selectedKey]);

  return style;
}
