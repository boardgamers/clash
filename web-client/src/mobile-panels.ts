/** Temporarily expose the board without unmounting panels or clearing their local form state. */
export function mobilePanels(
  root: HTMLElement,
  options: { key: string; onChange: (minimized: boolean, modal: boolean) => void },
) {
  const media = matchMedia('(max-width: 760px), (max-width: 1000px) and (max-height: 500px)');
  const selector = '.action-panel.floating-panel, .board-context, .activity.floating-panel, dialog';
  const attached = new Map<
    HTMLElement,
    { button: HTMLButtonElement; heading: HTMLElement | null; close: EventListener }
  >();
  const ignoredCloses = new WeakMap<HTMLDialogElement, number>();
  let minimized = false;
  let previous: HTMLElement | null = null;
  let dialogs: HTMLDialogElement[] = [];
  let scroll: { node: HTMLElement; top: number; left: number }[] = [];
  const restore = document.createElement('button');
  restore.className = 'restore-mobile-panel';
  restore.type = 'button';
  restore.hidden = true;
  root.append(restore);
  function title(panel: HTMLElement) {
    if (panel.matches('.board-context')) return 'Tile details';
    return (
      panel.querySelector('h2')?.textContent?.trim() ||
      panel.getAttribute('aria-label') ||
      'Panel'
    ).replace(/^City [A-Z]+\d+$/, 'City management');
  }
  function closeQuietly(dialog: HTMLDialogElement) {
    if (!dialog.open) return;
    ignoredCloses.set(dialog, (ignoredCloses.get(dialog) ?? 0) + 1);
    dialog.close();
  }
  function resume(focus = false) {
    if (!minimized) return;
    // A new dialog may have opened while the old one was minimized. Restore the
    // old stack underneath it, rather than stealing its focus or hiding its prompt.
    const newDialogs = [...root.querySelectorAll<HTMLDialogElement>('dialog[open]')];
    for (const dialog of newDialogs) closeQuietly(dialog);
    minimized = false;
    root.removeAttribute('data-panels-minimized');
    restore.hidden = true;
    for (const dialog of [...dialogs, ...newDialogs])
      if (dialog.isConnected && !dialog.open) dialog.showModal();
    for (const s of scroll) if (s.node.isConnected) s.node.scrollTo(s.left, s.top);
    options.onChange(false, false);
    if (focus && !newDialogs.length && previous?.isConnected)
      attached.get(previous)?.button.focus({ preventScroll: true });
    dialogs = [];
    scroll = [];
    previous = null;
  }
  function minimize(panel: HTMLElement) {
    if ((!media.matches && !panel.matches('.decision-panel')) || minimized) return;
    previous = panel;
    dialogs = [...root.querySelectorAll<HTMLDialogElement>('dialog[open]')];
    scroll = [
      ...root.querySelectorAll<HTMLElement>(
        `${selector}, ${selector
          .split(', ')
          .map((s) => `${s} *`)
          .join(', ')}`,
      ),
    ]
      .filter((node) => node.scrollTop || node.scrollLeft)
      .map((node) => ({ node, top: node.scrollTop, left: node.scrollLeft }));
    minimized = true;
    root.setAttribute('data-panels-minimized', '');
    restore.textContent = `↑ ${title(panel)}`;
    restore.setAttribute('aria-label', `Restore ${title(panel)}`);
    restore.hidden = false;
    for (const dialog of dialogs) closeQuietly(dialog);
    options.onChange(true, dialogs.length > 0);
    restore.focus({ preventScroll: true });
  }
  restore.onclick = () => resume(true);
  function scan() {
    for (const [panel, entry] of attached)
      if (!panel.isConnected) {
        panel.removeEventListener('close', entry.close, true);
        attached.delete(panel);
      }
    for (const panel of root.querySelectorAll<HTMLElement>(selector)) {
      if (attached.has(panel)) continue;
      const button = document.createElement('button');
      button.className = 'icon-button mobile-panel-minimize';
      button.type = 'button';
      button.title = 'Minimize to see the map';
      button.setAttribute('aria-label', 'Minimize to see the map');
      button.innerHTML =
        '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m6 9 6 6 6-6"/><path d="M5 20h14"/></svg>';
      button.onclick = (event) => {
        event.stopPropagation();
        minimize(panel);
      };
      const heading = panel.querySelector<HTMLElement>(':scope > header');
      if (heading) {
        const close = heading.querySelector<HTMLElement>('button[aria-label^="Close"]');
        if (close) close.before(button);
        else {
          const title = heading.querySelector('h2');
          if (title) title.after(button);
          else heading.append(button);
        }
      } else {
        panel.append(button);
        panel.classList.add('mobile-minimize-positioned');
        panel.classList.toggle(
          'mobile-minimize-has-close',
          !!panel.querySelector(':scope > button[aria-label^="Close"]'),
        );
      }
      panel.classList.add('mobile-minimizable');
      const close: EventListener = (event) => {
        const dialog = panel as HTMLDialogElement;
        const count = ignoredCloses.get(dialog) ?? 0;
        if (count) {
          ignoredCloses.set(dialog, count - 1);
          event.stopImmediatePropagation();
        }
      };
      panel.addEventListener('close', close, true);
      attached.set(panel, { button, heading, close });
    }
    if (minimized && !previous?.isConnected) resume();
  }
  const observer = new MutationObserver(scan);
  observer.observe(root, { childList: true, subtree: true });
  const resize = () => {
    if (!media.matches && !previous?.matches('.decision-panel')) resume();
  };
  const escape = (event: KeyboardEvent) => {
    if (minimized && event.key === 'Escape') {
      event.stopImmediatePropagation();
      event.preventDefault();
      resume(true);
    }
  };
  root.addEventListener('keydown', escape, true);
  media.addEventListener('change', resize);
  scan();
  return {
    update(next: typeof options) {
      const changed = next.key !== options.key;
      options = next;
      if (changed) resume();
    },
    destroy() {
      observer.disconnect();
      root.removeEventListener('keydown', escape, true);
      media.removeEventListener('change', resize);
      for (const [panel, entry] of attached) {
        panel.removeEventListener('close', entry.close, true);
        entry.button.remove();
      }
      restore.remove();
    },
  };
}
