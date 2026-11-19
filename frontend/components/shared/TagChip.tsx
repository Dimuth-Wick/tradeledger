interface Props {
  name: string;
  color: string;
  /** When given, the chip is a toggle button. */
  selected?: boolean;
  onClick?: () => void;
}

/** Small colored label for a tag. Clickable when `onClick` is set. */
export default function TagChip({ name, color, selected, onClick }: Props) {
  const style = { "--chip": color } as React.CSSProperties;
  if (!onClick) {
    return (
      <span className="chip" style={style}>
        {name}
      </span>
    );
  }
  return (
    <button
      type="button"
      className={`chip ${selected ? "on" : "off"}`}
      style={style}
      aria-pressed={selected}
      onClick={onClick}
    >
      {name}
    </button>
  );
}
