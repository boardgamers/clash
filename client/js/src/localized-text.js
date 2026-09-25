import { catalogs } from "./localization";
import { createTranslator, resolveLocale } from "./localization/runtime";

const translator = createTranslator(catalogs);
const canvas = document.createElement("canvas");
const context = canvas.getContext("2d", { willReadFrequently: true });
const fonts = new Map();
const measurements = new Map();
let revision = 0;
let locale = "en";
let family = "sans-serif";
let pendingLocale = 0;

async function loadFont(name, url) {
  if (!fonts.has(name))
    fonts.set(
      name,
      new FontFace(name, `url("${url}")`).load().then((font) => {
        document.fonts.add(font);
      }),
    );
  return fonts.get(name);
}
export async function setLocale(value, assets) {
  const requested = ++pendingLocale;
  const next = resolveLocale(value);
  const font =
    next === "hi"
      ? "Devanagari"
      : next === "ko"
        ? "Korean"
        : next === "zh-TW"
          ? "Chinese"
          : "SourceSans";
  const file =
    font === "SourceSans"
      ? "SourceSans3-Regular.ttf"
      : `localization/${font}.woff2`;
  await loadFont(`Clash${font}`, new URL(file, assets).href);
  if (requested !== pendingLocale) return;
  locale = next;
  family = `"Clash${font}", sans-serif`;
  translator.setLocale(locale);
  revision++;
  measurements.clear();
}
export function setPlayers(state) {
  try {
    const parsed = typeof state === "string" ? JSON.parse(state) : state;
    if (
      translator.setNames(
        (parsed?.players ?? []).flatMap((player) => [
          player.name,
          player.username,
        ]),
      )
    ) {
      revision++;
      measurements.clear();
    }
  } catch {
    /* Old engine snapshots may omit player names. */
  }
}
function metrics(text, size) {
  const translated = translator.translate(text);
  const key = `${size}:${translated}`;
  if (measurements.has(key)) return measurements.get(key);
  context.font = `${size}px ${family}`;
  const measured = context.measureText(translated);
  const left = Math.ceil(Math.max(0, measured.actualBoundingBoxLeft)) + 2;
  const ascent = Math.ceil(measured.actualBoundingBoxAscent) + 2;
  const width = Math.max(
    1,
    Math.ceil(
      left + Math.max(measured.width, measured.actualBoundingBoxRight),
    ) + 2,
  );
  const height = Math.max(
    1,
    Math.ceil(ascent + measured.actualBoundingBoxDescent) + 2,
  );
  const result = {
    translated,
    width,
    height,
    left,
    ascent,
    advance: measured.width,
    glyphHeight:
      measured.actualBoundingBoxAscent + measured.actualBoundingBoxDescent,
    glyphAscent: measured.actualBoundingBoxAscent,
  };
  if (measurements.size > 2000) measurements.clear();
  measurements.set(key, result);
  return result;
}
window.bgsTranslate = (text) => translator.translate(text);
window.bgsTextEnabled = () => locale !== "en";
window.bgsTextRevision = () => revision;
window.bgsTextMetrics = (text, size) => {
  const m = metrics(text, size);
  return new Float32Array([
    m.width,
    m.height,
    m.left,
    m.ascent,
    m.advance,
    m.glyphHeight,
    m.glyphAscent,
  ]);
};
window.bgsTextPixels = (text, size) => {
  const m = metrics(text, size);
  canvas.width = m.width * 2;
  canvas.height = m.height * 2;
  context.scale(2, 2);
  context.font = `${size}px ${family}`;
  context.fillStyle = "white";
  context.fillText(m.translated, m.left, m.ascent);
  return context.getImageData(0, 0, canvas.width, canvas.height).data;
};
window.bgsWrapText = (text, width, size) => {
  const translated = translator.translate(text);
  const words = new Intl.Segmenter(locale, { granularity: "word" }).segment(
    translated,
  );
  const lines = [];
  let line = "";
  for (const { segment } of words) {
    if (segment.includes("\n")) {
      lines.push(line.trimEnd());
      line = "";
      continue;
    }
    if (line && metrics(line + segment, size).advance > width) {
      lines.push(line.trimEnd());
      line = "";
    }
    if (metrics(segment, size).advance > width) {
      for (const { segment: character } of new Intl.Segmenter(locale, {
        granularity: "grapheme",
      }).segment(segment)) {
        if (line && metrics(line + character, size).advance > width) {
          lines.push(line);
          line = "";
        }
        line += character;
      }
    } else line += !line ? segment.trimStart() : segment;
  }
  if (line) lines.push(line.trimEnd());
  return JSON.stringify(lines);
};
