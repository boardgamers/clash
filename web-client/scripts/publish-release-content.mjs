import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {homedir} from 'node:os';
import assert from 'node:assert/strict';
const token=(await readFile(`${homedir()}/.bgs`,'utf8')).trim();
const base='https://admin.boardgamers.space/api/admin';
async function api(path,method='GET',body,type='application/json') {
 const response=await fetch(base+path,{method,headers:{Authorization:`Bearer ${token}`,'Content-Type':type},body:body===undefined?undefined:type==='application/json'?JSON.stringify(body):body});
 if(!response.ok)throw Error(`${path}: ${response.status}`);
 return response.json();
}
const before=await api('/gameinfo/clash/1');
await mkdir('.engine/releases',{recursive:true});
await writeFile(`.engine/releases/content-before-${Date.now()}.json`,JSON.stringify(before,null,2));
const rules=JSON.parse(await readFile('release/rules.json','utf8'));
for(const [locale,content] of Object.entries(rules)) {
 const path=`/page/clash%3Arules/${locale}`;
 await api(path,'PUT',{content,title:content.split('\n')[0].replace(/^# /,''),private:false});
 assert.equal((await api(path)).content,content);
}
const gallery=[];
for(const [name,caption] of [['board','Cities, armies and fleets'],['research','Choose your civilization’s research path'],['mobile','Strategy map on mobile']]) {
 for(const theme of ['light','dark'])await api(`/gameinfo/clash/assets/screenshot-release-${name}-${theme}`,'PUT',await readFile(`release/screenshots/${name}-${theme}.png`),'image/png');
 gallery.push({asset:`screenshot-release-${name}-light`,darkAsset:`screenshot-release-${name}-dark`,caption});
}
await api('/gameinfo/clash/gallery','PUT',{gallery});
const metadata=await api('/gameinfo/clash/meta');
await api('/gameinfo/clash/meta','PUT',{...metadata,rules:'[How to play](/page/clash/rules)',links:{...metadata.links,purchase:'https://shop.wizkids.com/products/clash-of-cultures-monumental-edition'}});
assert.deepEqual((await api('/gameinfo/clash/1')).gallery,gallery);
console.log('Published and verified 16 CMS rules pages and 3 screenshot pairs.');
