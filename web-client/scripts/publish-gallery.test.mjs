import { test } from 'node:test';
import assert from 'node:assert/strict';
import { replaceGallery, validateCaptures, unrelatedMetadata } from './publish-gallery.mjs';

test('all eight frozen PNGs and capture checks match the manifest', async () => {
  const images = await validateCaptures();
  assert.equal(images.filter(i => i.asset).length, 6);
  assert.equal(new Set(images.map(i => i.file)).size, 8);
});
test('replacement preserves ordering, captions, unrelated entries and extra fields', () => {
  const gallery = [{asset:'other', caption:'Keep'}, {asset:'old-light', darkAsset:'old-dark', caption:'Original', extra:true}];
  const images = [{previousAsset:'old-light',asset:'new-light'}, {previousAsset:'old-dark',asset:'new-dark'}];
  assert.deepEqual(replaceGallery(gallery, images), [gallery[0], {...gallery[1],asset:'new-light',darkAsset:'new-dark'}]);
  assert.equal(gallery[1].asset, 'old-light');
  assert.deepEqual(replaceGallery(replaceGallery(gallery, images), images), replaceGallery(gallery, images));
});
test('missing or duplicated target pairs fail closed', () => {
  const images = [{previousAsset:'old',asset:'new'}];
  assert.throws(() => replaceGallery([], images));
  assert.throws(() => replaceGallery([{asset:'old'}, {asset:'old'}], images));
});
test('metadata comparison excludes only gallery fields and server timestamp', () => {
  const before = {gallery:[],assets:{old:{}},updatedAt:'before',rules:'Keep',engine:{version:143},viewer:{url:'same'},options:[1]};
  const after = {...before,gallery:[{asset:'new'}],assets:{old:{},new:{}},updatedAt:'after'};
  assert.deepEqual(unrelatedMetadata(before), unrelatedMetadata(after));
  assert.notDeepEqual(unrelatedMetadata(before), unrelatedMetadata({...after,rules:'Changed'}));
  assert.notDeepEqual(unrelatedMetadata(before), unrelatedMetadata({...after,options:[]}));
});
