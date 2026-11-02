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
