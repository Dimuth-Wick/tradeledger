import { useRef } from "react";
import { useIndicator } from "../../logic/useIndicator";

interface Props<T extends string> {
  options: readonly (readonly [T, string])[];
  value: T;
  onChange: (value: T) => void;
  label: string;
}

/** A row of options with one chosen; the white chip slides to the choice. */
export default function Segmented<T extends string>({
  options,
  value,
  onChange,
  label,
}: Props<T>) {
  const row = useRef<HTMLDivElement>(null);
  const indicator = useIndicator(row, value);
  return (
    <div className="segmented" role="group" aria-label={label} ref={row}>
      <span className="segmented-indicator" style={indicator} aria-hidden />
      {options.map(([id, text]) => (
        <button
          key={id}
          type="button"
          aria-pressed={value === id}
          data-selected={value === id}
          className={value === id ? "active" : ""}
          onClick={() => onChange(id)}
        >
          {text}
        </button>
      ))}
    </div>
  );
}
