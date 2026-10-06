import { isPlayerSymbol } from '@boardgamers/protocol/player-symbols';
export function readPreferences(preferences: Record<string, unknown>) {
  const bgs = preferences.bgs as
    | {
        playerColors?: unknown;
        playerSymbols?: unknown;
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
    playerSymbols: Array.isArray(bgs?.playerSymbols)
      ? bgs.playerSymbols.map((s) => (isPlayerSymbol(s) ? s : ''))
      : [],
    playerBadges: (bgs?.players ?? []).map((player) => (player.pro ? bgs?.supporterBadge : undefined)),
    colorBlind: preferences.colorBlind === true,
    analysis: preferences.analysis === true,
    homeAtBottom: preferences.homeAtBottom === true,
    topDown: preferences.mapView === '2d' || preferences.mapView === 'strategy',
    strategyMap: preferences.mapView === '2d' || preferences.mapView === 'strategy',
    unitBadges: preferences.unitBadges === true,
    replayAutoplay: preferences.replayAutoplay !== false,
    availableOnly: preferences.availableOnly !== false,
  };
}
