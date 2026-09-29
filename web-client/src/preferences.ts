export function readPreferences(preferences: Record<string, unknown>) {
  return {
    locale: typeof preferences.locale === 'string' ? preferences.locale : 'en',
    sound: preferences.sound !== false,
    colorBlind: preferences.colorBlind === true,
    topDown: preferences.mapView === '2d',
    unitBadges: preferences.unitBadges !== false,
  };
}
