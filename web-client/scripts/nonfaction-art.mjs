// Original geometry, authored for Clash achievements. MIT (repository LICENSE).
// No external images, traced symbols, or fonts are used in the geometry.
export const modes = ['shorter', 'standard', 'roll-to-end', 'wonderful-ending', 'epic'];
export const wonders = [
  'arena',
  'pyramids',
  'gardens',
  'library',
  'lighthouse',
  'mausoleum',
  'statue',
  'wall',
];
export const ids = [
  ...modes.flatMap((m) => [`complete-${m}`, `win-${m}`]),
  'score-30',
  'score-40',
  'seven-cities',
  'twenty-advances',
  'three-objectives',
  'two-wonders-built',
  'captured-leader',
  ...wonders.map((w) => `built-great-${w}`),
  'complete-builder',
];
const path = (d) => `<path d="${d}"/>`;
const rect = (x, y, w, h) => `<rect x="${x}" y="${y}" width="${w}" height="${h}" rx="2"/>`;
const circle = (x, y, r) => `<circle cx="${x}" cy="${y}" r="${r}"/>`;
const group = (x, y, s, body) => `<g transform="translate(${x} ${y}) scale(${s})">${body}</g>`;
const city = () => path('M-15 16V-5L0-18 15-5V16Z M-5 16V3H5V16');
const check = () => path('M-13 0L-3 10 15-12');
const book = () => path('M-24-14Q-12-20 0-12Q12-20 24-14V17Q12 11 0 20Q-12 11-24 17Z M0-12V20');
export function monument(w) {
  switch (w) {
    case 'arena':
      return (
        '<ellipse cx="0" cy="15" rx="70" ry="34"/><ellipse cx="0" cy="-17" rx="70" ry="31"/>' +
        path('M-70-17V15 M70-17V15') +
        [-50, -25, 0, 25, 50].map((x) => path(`M${x - 6} 32V9Q${x} 0 ${x + 6} 9V32`)).join('')
      );
    case 'pyramids':
      return (
        path('M-75 40L-18-48 38 40Z M-18-48L-4 40 M10-5L38-43 79 40H38 M38-43L48 40') +
        path('M-50 10H-9 M-40-6H-12')
      );
    case 'gardens':
      return (
        rect(-69, 24, 138, 20) +
        rect(-52, -8, 104, 32) +
        rect(-30, -42, 60, 34) +
        [-52, -20, 18, 50]
          .map(
            (x, i) =>
              `<g fill="#67b5a1">${path(`M${x} ${i % 2 ? -28 : 5}q-20-23-24-2q3 14 24 10q18-26 28-5q-1 17-28 15v25`)}</g>`,
          )
          .join('')
      );
    case 'library':
      return (
        path('M-70-23L0-56 70-23Z') +
        rect(-65, -17, 130, 68) +
        [-51, -23, 5, 33].map((x) => rect(x, -8, 18, 48) + path(`M${x + 5}-2V33 M${x + 12}-2V33`)).join('') +
        rect(-73, 49, 146, 12)
      );
    case 'lighthouse':
      return (
        path('M-28 49L-20-36H20L28 49Z M-27-36H27L16-54H-16Z M-20-18H20 M-24 20H24') +
        rect(-7, -32, 14, 13) +
        `<g fill="#f7cc6c">${path('M27-29L76-48V-8Z')}</g>` +
        path('M-78 62Q-58 44-38 62T2 62T42 62T82 62')
      );
    case 'mausoleum':
      return (
        rect(-70, 42, 140, 15) +
        rect(-57, 26, 114, 16) +
        rect(-42, -25, 84, 51) +
        path('M-53-25L0-59 53-25Z M-27-14V18 M-9-14V18 M9-14V18 M27-14V18') +
        rect(-9, -73, 18, 14)
      );
    case 'statue':
      return (
        rect(-49, 42, 98, 20) +
        rect(-34, 28, 68, 14) +
        circle(0, -52, 13) +
        path('M-17-34L-30 6-15 10-10-9-16 28H17L10-9 22 1 39-28 28-34 15-17 13-34Z M-1-23V16')
      );
    case 'wall':
      return (
        path(
          'M-77 44V-7H-60V-22H-43V-7H-25V19L15-9V-35H32V-50H49V-35H67V14L25 32-19 59Z M-25 19V48 M15-9V22 M-60 5V24 M33-21V-2',
        ) + path('M-84 65L-50 54 M36 47L76 27')
      );
  }
}
function motif(id) {
  const mode = modes.find((m) => id === `complete-${m}` || id === `win-${m}`);
  if (mode) {
    const count = { shorter: 4, standard: 6, epic: 8 }[mode];
    let body = '';
    if (count)
      for (let i = 0; i < count; i++)
        body += group(72 + (i % 4) * 37, 100 + Math.floor(i / 4) * 43, 0.8, city());
    else {
      body = group(88, 112, 1.1, city()) + path('M110 112H136L128 104 M136 112L128 120');
      body +=
        mode === 'roll-to-end'
          ? rect(145, 86, 51, 51) +
            [
              [158, 99],
              [183, 99],
              [170, 112],
              [158, 125],
              [183, 125],
            ]
              .map(([x, y]) => circle(x, y, 3))
              .join('')
          : group(177, 112, 0.38, monument('mausoleum'));
    }
    if (id.startsWith('win-'))
      body +=
        `<g fill="#e9b34c">${path('M89 177L85 155 110 167 128 145 146 167 171 155 167 177Z')}</g>` +
        path('M63 158Q43 129 55 86 M193 158Q213 129 201 86');
    else body += group(128, 184, 0.8, check());
    return body;
  }
  if (id.startsWith('built-great-')) return group(128, 126, 1, monument(id.slice(12)));
  if (id === 'seven-cities')
    return (
      path('M72 90L128 66 184 90 184 150 128 178 72 150Z M72 90L128 121 184 90 M128 121V178') +
      [
        [128, 66],
        [72, 90],
        [184, 90],
        [128, 121],
        [72, 150],
        [184, 150],
        [128, 178],
      ]
        .map(([x, y]) => group(x, y, 0.65, city()))
        .join('')
    );
  if (id === 'twenty-advances')
    return (
      path('M128 174V71 M62 88H194 M62 118H194 M62 148H194') +
      [0, 1, 2, 3]
        .flatMap((row) => [0, 1, 2, 3, 4].map((col) => circle(62 + col * 33, 64 + row * 30, 7)))
        .join('') +
      group(128, 190, 0.6, book())
    );
  if (id === 'three-objectives')
    return [0, 1, 2]
      .map((i) =>
        group(
          64 + i * 64,
          118,
          1,
          rect(-24, -40, 48, 84) +
            group(0, -15, 0.55, [city(), book(), path('M-19 12L0-18 19 12Z')][i]) +
            group(0, 22, 0.55, check()),
        ),
      )
      .join('');
  if (id === 'two-wonders-built')
    return (
      group(82, 116, 0.55, monument('pyramids')) +
      group(174, 116, 0.55, monument('lighthouse')) +
      path('M47 171H209 M60 171V187 M196 171V187 M76 171V184 M180 171V184')
    );
  if (id === 'captured-leader')
    return (
      circle(128, 100, 21) +
      path('M91 153Q91 121 128 121Q165 121 165 153Z') +
      '<g fill="none">' +
      rect(72, 63, 112, 112) +
      '</g>' +
      path('M85 69V169 M171 69V169 M73 155H183') +
      rect(119, 165, 18, 23)
    );
  if (id === 'complete-builder')
    return (
      group(149, 143, 1.5, city()) +
      path('M69 177V69H172 M69 89L89 69 M149 69V94 M135 94H163V109H135Z M54 181H87') +
      group(129, 194, 0.65, check())
    );
  const high = id === 'score-40';
  return (
    group(high ? 91 : 89, 132, high ? 1.3 : 1.1, city()) +
    group(high ? 165 : 162, 127, high ? 0.55 : 0.45, monument(high ? 'library' : 'pyramids')) +
    group(128, 166, 0.65, book()) +
    (high ? path('M59 87H197 M68 80V94 M188 80V94') : '') +
    `<text x="128" y="211" text-anchor="middle" stroke="none" fill="#f7cc6c" font-family="sans-serif" font-weight="bold" font-size="29">${high ? '40' : '30'}</text>`
  );
}
export function svg(id) {
  if (!ids.includes(id)) throw Error('Unknown achievement');
  return `<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256" viewBox="0 0 256 256"><circle cx="128" cy="128" r="124" fill="#132e3b"/><circle cx="128" cy="128" r="114" fill="none" stroke="#b99050" stroke-width="3"/><g fill="#527b78" stroke="#f4dfb1" stroke-width="5" stroke-linejoin="round" stroke-linecap="round">${motif(id)}</g></svg>`;
}
