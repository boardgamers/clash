import init, { webQuery } from '../.bridge/server.js';
import { addEstimate, type CombatEstimate, type CombatScenario } from './combat-estimate';
self.onmessage = async ({
  data,
}: MessageEvent<{
  wasm: string;
  raw: string;
  seat: number;
  action: unknown;
  scenarios: CombatScenario[];
}>) => {
  try {
    await init({ module_or_path: await fetch(data.wasm) });
    const results: (CombatEstimate | undefined)[] = [];
    const errors: (string | undefined)[] = [];
    for (let offset = 0; offset < 2048; offset += 32) {
      for (let i = 0; i < data.scenarios.length; i++) {
        if (errors[i]) continue;
        try {
          const batch = JSON.parse(
            webQuery(
              data.raw,
              data.seat,
              JSON.stringify({
                kind: 'combatEstimate',
                action: data.action,
                ...data.scenarios[i],
                samples: 32,
                offset,
              }),
            ),
          ) as CombatEstimate;
          results[i] = addEstimate(results[i], batch);
        } catch (error) {
          results[i] = undefined;
          errors[i] = String(error);
        }
      }
      self.postMessage({
        results,
        errors,
        done: offset === 2016 || errors.filter(Boolean).length === data.scenarios.length,
      });
      if (errors.filter(Boolean).length === data.scenarios.length) break;
    }
  } catch (error) {
    self.postMessage({ error: String(error), done: true });
  }
};
