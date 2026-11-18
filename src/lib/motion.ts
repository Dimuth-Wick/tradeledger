// Shared motion settings, so every animation agrees and all of them stop
// when Windows "Show animations" is off (prefers-reduced-motion).

export function reducedMotion(): boolean {
  return (
    window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false
  );
}

/** Props for Recharts series: draw in on load, quick and eased. */
export function chartMotion(durationMs: number) {
  return {
    isAnimationActive: !reducedMotion(),
    animationDuration: durationMs,
    animationEasing: "ease-out" as const,
  };
}
