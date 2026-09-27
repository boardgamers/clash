export function bindPageScrolling(canvas) {
  // Miniquad cancels every touch, including swipes meant for the surrounding BGS page.
  // Let the browser scroll and generate compatibility mouse events for taps instead.
  const nativeTouch = (event) => event.stopImmediatePropagation();
  for (const type of ["touchstart", "touchmove", "touchend", "touchcancel"])
    canvas.addEventListener(type, nativeTouch, { capture: true, passive: true });
  canvas.style.touchAction = "pan-y pinch-zoom";
  canvas.addEventListener(
    "wheel",
    (event) => {
      if (!event.ctrlKey && !event.metaKey && !document.fullscreenElement)
        event.stopImmediatePropagation();
    },
    { capture: true, passive: true },
  );
}
