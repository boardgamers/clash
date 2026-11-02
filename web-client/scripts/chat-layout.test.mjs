// Run explicitly with: node --test scripts/chat-layout.test.mjs
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { chromium } from 'playwright';

const css = await readFile(new URL('../src/style.css', import.meta.url), 'utf8');
const protocolSource = await readFile(
  new URL('../node_modules/@boardgamers/protocol/dist/chat-dom.js', import.meta.url), 'utf8',
);
const protocolCss = protocolSource.split('const styles = `')[1].split('`;')[0];

test('chat composer stays at the bottom and close/minimize controls do not overlap', async () => {
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    assert(await page.evaluate(() => CSS.supports('selector(::details-content)')));
    for (const width of [390, 1400]) {
      for (const count of [0, 2, 60]) {
        await page.setViewportSize({ width, height: 800 });
        await page.setContent(`<style>${protocolCss}\n${css}</style>
          <div class="game-shell"><section class="activity floating-panel mobile-minimize-positioned mobile-minimize-has-close"
            style="position:absolute;top:10px;left:10px;width:360px;height:500px">
            <button class="mobile-panel-minimize">−</button>
            <div class="activity-tabs"><button>Journal</button><button>Chat</button>
              <button class="icon-button close-activity">×</button></div>
            <div class="chat-panel"><details open class="bgs-game-chat"><summary>Chat</summary>
              <div class="chat-messages"><div class="chat-contents">${'<article>Message</article>'.repeat(count)}</div></div>
              <div class="chat-composer"><input><button>Send</button></div><div class="chat-status"></div>
            </details></div>
          </section></div>`);
        const boxes = await page.evaluate(() => {
          const rect = (selector) => document.querySelector(selector).getBoundingClientRect().toJSON();
          return {
            panel: rect('.activity'), composer: rect('.chat-composer'), close: rect('.close-activity'),
            minimize: rect('.mobile-panel-minimize'), messages: rect('.chat-messages'),
            scroll: document.querySelector('.chat-messages').scrollHeight,
          };
        });
        const context = JSON.stringify({ width, count, boxes });
        assert(boxes.panel.bottom - boxes.composer.bottom < 35, context);
        assert(boxes.composer.bottom <= boxes.panel.bottom, context);
        assert(boxes.panel.right - boxes.close.right < 8, context);
        assert(boxes.close.top - boxes.panel.top < 3, context);
        assert.equal(boxes.close.width, 44, context);
        assert.equal(boxes.close.height, 44, context);
        assert(boxes.messages.bottom <= boxes.composer.top, context);
        if (width === 390) assert(boxes.minimize.right < boxes.close.left, context);
        if (count === 60) assert(boxes.scroll > boxes.messages.height, context);
      }
    }
    // Simulate an older browser rejecting the feature query: keep its scroll cap.
    await page.setContent(`<style>${protocolCss}\n${css.replace('@supports selector(::details-content)', '@supports (unknown-chat-feature: yes)')}</style>
      <div class="chat-panel" style="height:400px"><details open class="bgs-game-chat">
        <div class="chat-messages">${'<article>Message</article>'.repeat(60)}</div>
        <div class="chat-composer"><input><button>Send</button></div>
      </details></div>`);
    assert.notEqual(await page.locator('.chat-messages').evaluate((el) => getComputedStyle(el).maxHeight), 'none');
  } finally {
    await browser.close();
  }
});
