export function readPreferences(preferences: Record<string, unknown>) {
  const bgs = preferences.bgs as
    | {
        playerColors?: unknown;
        players?: { pro?: boolean }[];
        supporterBadge?: { url: string; label: string };
      }
    | undefined;
  const playerColors = Array.isArray(bgs?.playerColors)
    ? bgs.playerColors.map((c) => (typeof c === 'string' && /^#[a-f0-9]{6}$/i.test(c) ? c : ''))
    : [];
  return {
    locale: typeof preferences.locale === 'string' ? preferences.locale : 'en',
    sound: preferences.sound !== false,
    playerColors,
    playerBadges: (bgs?.players ?? []).map((player) => (player.pro ? bgs?.supporterBadge : undefined)),
    colorBlind: preferences.colorBlind === true,
    topDown: preferences.mapView === '2d',
    unitBadges: preferences.unitBadges === true,
  };
}
