<script lang="ts">
  import { onMount } from 'svelte';
  import App from './App.svelte';
  import { Controller } from './controller';
  import { loadTutorialEngine } from './bridge';
  import { mountLocalization } from './localization';
  import fixture from '../scripts/async-preview-fixture.json';
  import './style.css';
  import { ActionQueue } from './action-queue';
  let seat = $state(0);
  let pending = $state(true);
  let waiting = $state(false);
  let error = $state('');
  let ready = $state(false);
  let engine: Awaited<ReturnType<typeof loadTutorialEngine>>;
  let game: any;
  let queue: ActionQueue;
  let queued = $state(0);
  let ended = $state(false);
  let remaining = $state<unknown[]>([]);
  function describeAction(action: any): string {
    if(action?.Playing === 'EndTurn') return 'End turn';
    const playing=action?.Playing;
    if(playing?.Collect) return `Collect in ${playing.Collect.city_position}`;
    if(playing?.Advance) return `Research ${playing.Advance.advance}`;
    if(action?.Response) return 'Confirm action choice or payment';
    const key=Object.keys(playing ?? action ?? {})[0] ?? 'Action';
    return key.replace(/([a-z])([A-Z])/g,'$1 $2');
  }
  function syncQueue() {queued=queue.count;ended=queue.ended;}
  async function undoQueued() {queue.undo();syncQueue();await refresh();}

  let controller = $state<Controller>();
  const names = fixture.players.slice(0,2).map(p => p.civilization);
  function snapshot() {
    if (pending && seat === 0 && !waiting) return JSON.parse(queue.projected);
    return pending ? {...game, events: structuredClone(fixture.events)} : game;
  }
  async function refresh() {
    controller!.setPlayer(seat);
    await controller!.load(engine.stripSecret(JSON.stringify(snapshot()),seat));
  }
  async function move(raw: string) {
    try {
      error='';
      if(pending && seat === 0) {
        if(waiting) throw Error('Waiting for the replacement settler.');
        const action=JSON.parse(raw);
        if(action==='Undo') {queue.undo();syncQueue();await refresh();return;}
        const result=queue.append(action);
        if(!result.ok) throw Error(result.reason);
        syncQueue();
        // End turn has no speculative state change to acknowledge.
        if(ended) controller!.moveResult({ok:false,error:''});
        await refresh();
        controller!.patch({error:''});
        return;
      }
      const result = JSON.parse(engine.tryMove(JSON.stringify(snapshot()),raw,seat));
      if(pending && seat === 1) {
        // The fixture's reaction resolves against the untouched authoritative board.
        game=result;
        pending=false; waiting=false;
        const replay=queue.replay(JSON.stringify(game));
        game=JSON.parse(replay.raw);
        remaining=replay.remaining;
        if(replay.error) error=`Queue stopped after ${replay.applied} actions. ${replay.error}`;
        queued=0;ended=false;
      } else game=result;
      await refresh();
    } catch(e) { error=String(e); controller!.moveResult({ok:false,error}); }
  }
  async function restart() {
    game=structuredClone(fixture);
    game.events=[]; game.state='Playing'; game.actions_left=3;
    for(const city of game.players[0].cities) {city.activations=0;city.mood_state='Happy';}
    game.players[0].resources.food=2;
    game.board_history={id:'async-queue-preview',frames:[]};
    queue=new ActionQueue(engine,0,JSON.stringify(game));
    syncQueue();remaining=[];
    pending=true; waiting=false; error=''; seat=0;
    await refresh();
    ready=true;
  }
  async function switchSeat(value:number) {seat=value;await refresh();controller!.patch({mode:'overview',error:''});}
  onMount(()=>{
    const localization=mountLocalization(document.body);
    controller=new Controller({
      move(raw){void move(raw);return true;},
      fetchState(){void refresh();return true;},
      updatePreference(name,value){controller!.setPreferences({mapView:'3d',[name]:value});return true;},
      updateSetting(){return true;},openPlayer(){return true;},hoverPlayer(){return true;},leavePlayer(){return true;},openBoardgame(){return true;},fetchLog(){return true;},addLog(){return true;},replaceLog(){return true;},setReplayInfo(){return true;},
    },new URL('/',location.href),{predict:false});
    controller.setPreferences({mapView:'3d',locale:'en',sound:false});
    void (async()=>{await localization.ready;engine=await loadTutorialEngine();await restart();})().catch(e=>error=String(e));
    return ()=>{controller?.destroy();localization.destroy();};
  });
</script>
<div class="prototype-tools">
  <span>Local prototype · Replacement settler</span>
  <div><button class:chosen={seat===0} onclick={()=>switchSeat(0)}>Play as {names[0]}</button><button class:chosen={seat===1} onclick={()=>switchSeat(1)}>Play as {names[1]}</button><button onclick={restart}>Restart</button></div>
</div>
{#if error}<div class="prototype-error">{error}<button onclick={()=>error=''} aria-label="Dismiss error">×</button></div>{/if}
{#if remaining.length}<aside class="queue-review"><strong>{remaining.length} queued actions need review</strong>{#each remaining as action}<p>{describeAction(action)}</p>{/each}<button onclick={()=>remaining=[]}>Discard remaining queue</button></aside>{/if}
<div class="real-game">
  {#if controller && ready}<App {controller} concurrentPreview={{pending,waiting,seat,other:names[1],queued,ended,onUndo:undoQueued,onWait:async()=>{waiting=true;await refresh();}}}/>{:else}<p>Loading the game…</p>{/if}
</div>
<style>
  :global(html),:global(body),:global(#app){margin:0;width:100%;height:100%;overflow:hidden;}
  .prototype-tools{height:46px;box-sizing:border-box;display:flex;align-items:center;justify-content:space-between;gap:8px;padding:6px 12px;background:#1c303b;color:#e5edf0;font:12px system-ui;}
  .prototype-tools div{display:flex;gap:5px;}.prototype-tools button{border:1px solid #708a92;border-radius:4px;padding:6px 10px;background:transparent;color:inherit;cursor:pointer;}.prototype-tools button.chosen{background:#306b75;border-color:#b3d4db;}
  .queue-review{position:fixed;z-index:10001;left:12px;bottom:60px;max-height:40vh;overflow:auto;background:#f5f8f9;color:#263c48;padding:16px;border-radius:6px;}
  .real-game :global(.game-shell){height:100%;min-height:0;}
  .real-game{position:relative;height:calc(100% - 46px);width:100%;}.prototype-error{position:fixed;z-index:10000;bottom:10px;left:10px;background:#642e29;color:white;padding:12px;}
  @media(max-width:600px){.prototype-tools{height:70px;flex-wrap:wrap;}.real-game{height:calc(100% - 70px);}}
</style>
