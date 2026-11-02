// Keep selector/key heuristics for strings in code, but a Svelte Text node is
// already known to be visible. Leading punctuation can be ordinary prose there.
export function sourceMessage(value, { visibleText = false } = {}) {
  const text = value.replace(/\s+/g, ' ').trim();
  if (
    !/[A-Za-z]/.test(text) ||
    text.length < 2 ||
    /\{p\d+\}\{p\d+\}/.test(text) ||
    /^[-a-z0-9_:/]+$/.test(text) ||
    /[a-z]_[a-z]/.test(text) ||
    /^[a-z]+(?:[A-Z][a-z0-9]*)+$/.test(text) ||
    /^(?:[a-z]+-)+[a-z]+(?:\s+[a-z-]+)*$/.test(text) ||
    /^(?:[a-z]+-)+.*\{p\d+\}/.test(text) ||
    /^(?:crate::|three\/|position:|Building:|Wonder:|pending:|objective:|person:|status:)/.test(text) ||
    /\{p\d+\}[:|](?:\{p\d+\}|[a-zA-Z])/.test(text) ||
    /^\$\d/.test(text) ||
    /^(?:Arrow(?:Up|Down|Left|Right)|F\d+|[A-Z][a-z]+(?:[A-Z][a-z]+)+)$/.test(text) ||
    (!visibleText && /^[#.:\[]/.test(text)) ||
    /^[a-z][a-z0-9-]*\[[a-z-]+(?:[=\]^$*~|]|\])/.test(text) ||
    /^<[a-z]/.test(text) ||
    /[A-Za-z]__[A-Za-z]/.test(text) ||
    /=>|\\[bpdw]|\.\/(?:src|assets)|https?:|\.svg|\.wasm|rgba?\(|gradient\(|var\(|\d(?:px|rem)\b/.test(text)
  ) return null;
  return text;
}

// Preserve whether a candidate came from visible markup rather than a string
// literal (which may be a selector or another technical value in a script).
export function* svelteStrings(node) {
  if (!node || typeof node !== 'object') return;
  // Only human-facing attributes; class names, event handlers and IDs are not messages.
  if (node.type === 'Attribute' && !['title', 'aria-label', 'placeholder', 'alt', 'label'].includes(node.name)) return;
  if (
    node.type === 'Fragment' &&
    node.nodes?.some((n) => n.type === 'ExpressionTag') &&
    node.nodes.every((n) => ['Text', 'ExpressionTag', 'Comment'].includes(n.type))
  ) {
    let index = 0;
    yield {
      value: node.nodes
        .map((n) => (n.type === 'Text' ? n.data : n.type === 'ExpressionTag' ? `{p${index++}}` : ''))
        .join(''),
      visibleText: true,
    };
  }
  if (node.type === 'Text') yield { value: node.data, visibleText: true };
  if (node.type === 'Literal' && typeof node.value === 'string') yield { value: node.value };
  if (node.type === 'TemplateLiteral') yield {
    value: node.quasis.map((q, i) => q.value.cooked + (i < node.expressions.length ? `{p${i}}` : '')).join(''),
  };
  for (const [key, value] of Object.entries(node)) {
    if (['css', 'start', 'end', 'loc'].includes(key)) continue;
    if (Array.isArray(value)) {
      for (const child of value) yield* svelteStrings(child);
    } else if (value && typeof value === 'object') yield* svelteStrings(value);
  }
}

// Rust strings are not JSON strings: escaped newlines consume the following
// whitespace, and Rust also supports raw strings, \0, \xNN and \u{NNNN}.
export function decodeRustString(body) {
  return body.replace(
    /\\(?:(\r?\n\s*)|u\{([\da-fA-F_]+)\}|x([\da-fA-F]{2})|([\\"'nrt0]))/g,
    (_, continuation, unicode, hex, escape) => continuation ? ''
      : unicode ? String.fromCodePoint(parseInt(unicode.replaceAll('_', ''), 16))
        : hex ? String.fromCodePoint(parseInt(hex, 16))
          : ({ n: '\n', r: '\r', t: '\t', 0: '\0' }[escape] ?? escape),
  );
}

export function rustStrings(content) {
  const result = [];
  // Lex comments along with literals so quoted examples in comments never count.
  const token = /'(?:\\(?:u\{[\da-fA-F_]+\}|x[\da-fA-F]{2}|.)|[^'\\])'|\/\/[^\n]*|\/\*|r(#+)?"|"/g;
  let match;
  while ((match = token.exec(content))) {
    const start = match.index;
    if (match[0].startsWith('//') || match[0].startsWith("'")) continue;
    if (match[0] === '/*') {
      let depth = 1;
      const comments = /\/\*|\*\//g;
      comments.lastIndex = token.lastIndex;
      let next;
      while (depth && (next = comments.exec(content))) depth += next[0] === '/*' ? 1 : -1;
      token.lastIndex = depth ? content.length : comments.lastIndex;
      continue;
    }
    const raw = match[0].startsWith('r');
    const begin = token.lastIndex;
    let end;
    if (raw) {
      const closing = '"' + (match[1] ?? '');
      end = content.indexOf(closing, begin);
      if (end < 0) break;
      token.lastIndex = end + closing.length;
    } else {
      end = begin;
      while (end < content.length) {
        if (content[end] === '\\') end += 2;
        else if (content[end] === '"') break;
        else end++;
      }
      token.lastIndex = end + 1;
    }
    result.push({ value: raw ? content.slice(begin, end) : decodeRustString(content.slice(begin, end)), start });
  }
  return result;
}

export function rustTemplate(value) {
  let index = 0;
  return value.replace(/\{\{|\}\}|\{(?:[a-zA-Z_][a-zA-Z_0-9]*|\d+)?(?::[^}]*)?\}/g,
    (match) => match === '{{' ? '{' : match === '}}' ? '}' : `{p${index++}}`);
}
