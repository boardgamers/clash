// Synthetic public metadata, not a captured player/account payload. Badge images
// use HTTP(S), matching the platform fix: emit assets, not inlined SVG data URLs.
export function hostPreferences(mapView = '3d', count = 3) {
  const badge = (id, url) => ({
    id,
    label: `Badge ${id}`,
    shortLabel: id,
    description: 'Public achievement',
    url,
  });
  const safe = badge('supporter', 'https://assets.example.test/supporter.svg');
  return {
    locale: 'en',
    sound: false,
    replayAutoplay: false,
    mapView,
    homeAtBottom: true,
    platformChat: true,
    preferredPlayerColors: Array(count).fill(null),
    bgs: {
      players: Array.from({ length: count }, (_, seat) => ({
        pro: seat === 1,
        badges:
          seat === 1
            ? [safe, badge('achievement', 'https://assets.example.test/achievement.svg')]
            : [badge('achievement', 'https://assets.example.test/achievement.svg')],
      })),
      playerColors: ['#ff0000', '#00ff00', '#0000ff', '#ffff00'].slice(0, count),
      supporterBadge: { label: 'Supporter', url: safe.url },
    },
  };
}
