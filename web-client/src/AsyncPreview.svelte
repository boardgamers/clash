<script lang="ts">
  import { onMount } from 'svelte';
  import App from './App.svelte';
  import { Controller } from './controller';
  import { loadTutorialEngine } from './bridge';
  import { mountLocalization } from './localization';
  import settlerFixture from '../scripts/async-preview-fixture.json';
  import tradeFixture from '../scripts/tech-queue-fixture.json';
  import './style.css';
  let scenario=$state<'settler'|'trade'>('trade');
  let seat=$state(0), ready=$state(false), error=$state('');
  let engine: Awaited<ReturnType<typeof loadTutorialEngine>>;
  let game:any;
  let controller=$state<Controller>();
  let names=$derived((scenario==='settler'?settlerFixture:tradeFixture).players.slice(0,2).map(p=>p.civilization));
  async function refresh(){controller!.setPlayer(seat);await controller!.load(engine.stripSecret(JSON.stringify(game),seat));}
  async function move(raw:string){
    try{game=JSON.parse(engine.tryMove(JSON.stringify(game),raw,seat));error='';await refresh();}
    catch(e){error=String(e);controller!.moveResult({ok:false,error});}
  }
  async function restart(next=scenario){
    scenario=next;game=structuredClone(next==='settler'?settlerFixture:tradeFixture);
    game.board_history??={id:'async-queue-preview',frames:[]};
    for(const c of game.players[0].cities){c.activations=0;c.mood_state='Happy';}
    game.players[0].resources.food=1;seat=0;error='';await refresh();ready=true;
  }
  async function switchSeat(next:number){seat=next;await refresh();controller!.patch({mode:'overview',error:''});}
  onMount(()=>{
    const localization=mountLocalization(document.body);
    controller=new Controller({move(raw){void move(raw);return true;},fetchState(){void refresh();return true;},
      updatePreference(name,value){controller!.setPreferences({mapView:'3d',[name]:value});return true;},
      updateSetting(){return true;},openPlayer(){return true;},hoverPlayer(){return true;},leavePlayer(){return true;},openBoardgame(){return true;},fetchLog(){return true;},addLog(){return true;},replaceLog(){return true;},setReplayInfo(){return true;},
    },new URL('/',location.href),{predict:false});
    controller.setPreferences({mapView:'3d',locale:'en',sound:false});
    void(async()=>{await localization.ready;engine=await loadTutorialEngine();await restart();})().catch(e=>error=String(e));
    return()=>{controller?.destroy();localization.destroy();};
  });
</script>

<div class="prototype-tools">
  <span>Local test · Live queue implementation</span>
  <div><button class:chosen={scenario==='settler'} onclick={()=>restart('settler')}>Replacement settler</button><button class:chosen={scenario==='trade'} onclick={()=>restart('trade')}>Technology Trade</button></div>
  <div><button class:chosen={seat===0} onclick={()=>switchSeat(0)}>Play as {names[0]}</button><button class:chosen={seat===1} onclick={()=>switchSeat(1)}>Play as {names[1]}</button><button onclick={()=>restart()}>Restart</button></div>
</div>
{#if error}<div class="prototype-error">{error}<button onclick={()=>error=''}>×</button></div>{/if}
<div class="real-game">{#if controller && ready}<App {controller}/>{:else}<p>Loading the game…</p>{/if}</div>
<style>
  :global(html),:global(body),:global(#app){margin:0;width:100%;height:100%;overflow:hidden;}
  .prototype-tools{height:46px;box-sizing:border-box;display:flex;align-items:center;justify-content:space-between;gap:8px;padding:6px 12px;background:#1c303b;color:#e5edf0;font:12px system-ui;}
  .prototype-tools div{display:flex;gap:5px;}.prototype-tools button{border:1px solid #708a92;border-radius:4px;padding:6px 10px;background:transparent;color:inherit;cursor:pointer;}.prototype-tools button.chosen{background:#306b75;border-color:#b3d4db;}
  .real-game :global(.game-shell){height:100%;min-height:0;}
  .real-game{position:relative;height:calc(100% - 46px);width:100%;}.prototype-error{position:fixed;z-index:10000;bottom:10px;left:10px;background:#642e29;color:white;padding:12px;}
  @media(max-width:600px){.prototype-tools{height:70px;flex-wrap:wrap;}.real-game{height:calc(100% - 70px);}}
</style>
