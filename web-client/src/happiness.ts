import type { HappinessPreview, Session } from './types';

type Query = <T>(input: unknown) => T;

export function happinessPreview(
  session: Session,
  query: Query,
  steps = session.happinessSteps ?? {},
  lawgiverCity = session.happinessLawgiver ?? null,
): HappinessPreview {
  const cities = Object.entries(steps).filter(([, n]) => n > 0);
  const variant = session.view?.happinessActions?.[session.happinessVariant ?? 0];
  if (!cities.length || !variant) return { action: null, payment: {}, reason: null };
  try {
    return query<HappinessPreview>({
      kind: 'happiness',
      cities,
      variant: variant.value,
      lawgiver: cities.some(([position]) => position === lawgiverCity),
    });
  } catch (error) {
    return { action: null, payment: {}, reason: String(error) };
  }
}

export function happinessTargets(session: Session, query: Query, position: string) {
  return (session.view?.cityActions.find((c) => c.position === position)?.happiness ?? []).map((target) => {
    const lawgiver = target.lawgiver
      ? position
      : session.happinessLawgiver === position
        ? null
        : session.happinessLawgiver;
    return {
      ...target,
      ...happinessPreview(session, query, { ...session.happinessSteps, [position]: target.steps }, lawgiver),
    };
  });
}

export function happinessCities(session: Session) {
  return session.view?.cities.filter((city) => city.mood !== 'Happy').map((city) => city.position) ?? [];
}
