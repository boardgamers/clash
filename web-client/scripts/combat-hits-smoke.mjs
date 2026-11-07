import assert from 'node:assert/strict';
import { createServer } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { chromium } from 'playwright';

// Both real views consume the same synthetic public journal projection.
const server = await createServer({
  configFile: false,
  appType: 'custom',
  root: new URL('..', import.meta.url).pathname,
  plugins: [svelte()],
  server: { host: '127.0.0.1', port: 0 },
});
server.middlewares.use(async (req, res, next) => {
  if (req.url !== '/hits') return next();
  res.setHeader('Content-Type', 'text/html');
  res.end(
    await server.transformIndexHtml(
      req.url,
      `<!doctype html><meta name="viewport" content="width=device-width,initial-scale=1">
    <style>:root { --muted:#777; --ink:#222; --paper2:#fff; --line:#ccc; } #journal { max-width:600px; }</style>
    <div id="journal"></div><div class="play-layout"><div class="live-battle" id="popup"></div></div>
    <script type="module">
      import { mount, unmount } from 'svelte';
      import { writable } from 'svelte/store';
      import CombatJournal from '/src/CombatJournal.svelte';
      import BattlePlayback from '/src/BattlePlayback.svelte';
      import { combatJournal } from '/src/combat-journal.ts';
      import '/src/combat.css';
      const session = writable({battles: []});
      mount(BattlePlayback, {target: document.querySelector('#popup'), props: {controller:{session, dismissBattles(){}}}});
      let journal;
      window.showHits = async (fighters, cancels, late, extra = 0, value = 29) => {
        const entries = [];
        const add = (player, title, notes) => entries.push({id:'0-0-0-0-'+entries.length, age:1, round:1, kind:'combat', title, player, civilization:player ? 'China' : 'Celts', notes:[notes], tokens:[], text:''});
        add(undefined, '', 'Combat round 1');
        add(0, 'Combat', 'Attacking with 2 infantry and Vercingetorix');
        add(1, 'Combat', 'Defending with '+fighters+' infantry and 2 settlers');
        add(0, 'Combat', 'Roll 6 → combat value '+value+' → '+Math.min(fighters, Math.max(0, Math.floor(value/5)-cancels)+extra)+' hits against defending units');
        add(0, 'Combat', 'Hit report: value '+value+'; cancels '+cancels+'; extra '+extra+'; casualties '+Math.min(fighters, Math.max(0, Math.floor(value/5)-cancels)+extra));
        if(late) add(1, 'Tactician', 'Ignore the first hit');
        const combat = combatJournal(entries).find(e => e.combat).combat;
        if(journal) await unmount(journal);
        journal = mount(CombatJournal, {target:document.querySelector('#journal'), props:{combat}});
        session.set({battles:[{key:JSON.stringify(combat), roll:false, losses:[], combat}], battleAnimate:false, reducedMotion:true});
      };
    </script>`,
    ),
  );
});
await server.listen();
const browser = await chromium.launch({ headless: true });
try {
  for (const width of [1400, 390, 320]) {
    const page = await browser.newPage({ viewport: { width, height: 900 } });
    const errors = [];
    page.on('pageerror', (e) => errors.push(e.message));
    await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/hits`);
    await page.waitForFunction(() => typeof window.showHits === 'function');
    for (const [fighters, cancels, late, extra, value, before, nominal, casualties] of [
      [4, 2, false, 0, 29, 5, 3, 3],
      [4, 2, true, 0, 29, 5, 2, 2],
      [3, 1, false, 0, 29, 5, 4, 3],
      [4, 1, false, 0, 29, 5, 4, 4],
      [3, 0, false, 0, 29, null, 5, 3],
      [3, 2, false, 0, 0, null, 0, 0],
      [4, 2, false, 1, 29, 6, 4, 4],
    ]) {
      await page.evaluate((args) => window.showHits(...args), [fighters, cancels, late, extra, value]);
      const cell = page.locator('#journal .combat-hits td').first();
      const popup = page.locator('.battle-hit-count strong').first();
      assert.equal(await cell.locator('b').innerText(), String(nominal));
      assert.match(await popup.innerText(), new RegExp(`${nominal} hits?$`));
      for (const view of [cell, popup]) {
        assert.equal(await view.locator('s').count(), before === null ? 0 : 1);
        if (before !== null) {
          assert.equal(await view.locator('s').innerText(), String(before));
          assert.match(
            await view.locator('s').evaluate((el) => getComputedStyle(el).textDecorationLine),
            /line-through/,
          );
        }
      }
      assert.equal(await page.locator('#journal .combat-casualties').count(), nominal !== casualties ? 1 : 0);
      assert.equal(await page.locator('.battle-casualties').count(), nominal !== casualties ? 1 : 0);
      if (nominal !== casualties) {
        assert.equal(
          await page.locator('#journal .combat-casualties td').first().innerText(),
          String(casualties),
        );
        assert.equal(await page.locator('.battle-casualties').innerText(), `Casualties: ${casualties}`);
      }
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    }
    assert.deepEqual(errors, []);
    await page.close();
    console.log(
      `${width}px: journal and popup show uncapped cancellations, paid changes, extras and separate casualties.`,
    );
  }
} finally {
  await browser.close();
  await server.close();
}
