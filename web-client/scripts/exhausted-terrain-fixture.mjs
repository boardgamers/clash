/** Real engine state for comparing normal and exhausted terrain in the normal viewer. */
export async function exhaustedTerrainFixture(engine) {
  const game = JSON.parse(await engine.init(2, [], {}, 'exhausted-terrain-preview', {}));
  const occupied = new Set(
    game.players.flatMap((p) => [
      ...(p.cities ?? []).map((c) => c.position),
      ...(p.units ?? []).map((u) => u.position),
    ]),
  );
  const tiles = game.map.tiles.filter(([p]) => !occupied.has(p));
  const kinds = ['Forest', 'Mountain', 'Fertile', 'Barren'];
  game.map.unexplored_blocks = [];
  for (const tile of game.map.tiles) if (tile[1] === 'Unexplored') tile[1] = 'Barren';
  const examples = [];
  for (let i = 0; i < kinds.length; i++) {
    const normal = tiles[i * 2],
      exhausted = tiles[i * 2 + 1];
    normal[1] = kinds[i];
    exhausted[1] = { Exhausted: kinds[i] };
    examples.push({ kind: kinds[i], normal: normal[0], exhausted: exhausted[0] });
  }
  delete game.board_history;
  const state = JSON.stringify(game),
    seat = game.current_player_index;
  engine.webView(engine.stripSecret(state, seat), seat);
  return { state, seat, examples };
}
