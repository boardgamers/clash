// Keep the legend inside the board and clear of the Journal/Chat toolbar.
export function positionStrategyKey(node: HTMLDetailsElement) {
  const content = node.querySelector<HTMLElement>('.strategy-key-content')!;
  const controls = node.closest<HTMLElement>('.map-controls')!;
  const board = node.closest<HTMLElement>('.map-section')!;
  const tableTools = node.closest('.play-layout')!.querySelector<HTMLElement>('.table-tools')!;
  const gap = 8;
  const position = () => {
    if (!node.open) return;
    const bounds = board.getBoundingClientRect();
    const anchor = controls.getBoundingClientRect();
    const toolbar = tableTools.getBoundingClientRect();
    const width = Math.min(300, bounds.width - gap * 2);
    const left = Math.max(bounds.left + gap, Math.min(anchor.right - width, bounds.right - width - gap));
    content.style.width = `${width}px`;
    let spaces = [
      { top: bounds.top + gap, bottom: anchor.top - gap, above: true },
      { top: anchor.bottom + gap, bottom: bounds.bottom - gap, above: false },
    ];
    if (toolbar.width && left < toolbar.right && left + width > toolbar.left)
      spaces = spaces.flatMap((space) => [
        { ...space, bottom: Math.min(space.bottom, toolbar.top - gap) },
        { ...space, top: Math.max(space.top, toolbar.bottom + gap) },
      ]);
    spaces = spaces.filter((space) => space.bottom > space.top);
    if (!spaces.length) return;
    const height = content.scrollHeight + content.offsetHeight - content.clientHeight;
    const space =
      spaces.find((space) => space.bottom - space.top >= height) ??
      spaces.reduce((a, b) => (a.bottom - a.top >= b.bottom - b.top ? a : b));
    const available = space.bottom - space.top;
    content.style.maxHeight = `${available}px`;
    content.style.left = `${left - anchor.left - controls.clientLeft}px`;
    content.style.top = `${(space.above ? space.bottom - Math.min(height, available) : space.top) - anchor.top - controls.clientTop}px`;
  };
  const observer = new ResizeObserver(position);
  for (const element of [board, controls, tableTools, content]) observer.observe(element);
  node.addEventListener('toggle', position);
  window.addEventListener('resize', position);
  return {
    destroy() {
      observer.disconnect();
      node.removeEventListener('toggle', position);
      window.removeEventListener('resize', position);
    },
  };
}
