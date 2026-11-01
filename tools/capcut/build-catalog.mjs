import { DatabaseSync } from 'node:sqlite';
import fs from 'node:fs';

const DB = 'C:\\Users\\aitor\\AppData\\Local\\CapCut\\User Data\\Cache\\ressdk_db\\1066174546386837715\\rp.db';
const db = new DatabaseSync(DB, { readOnly: true });
const rows = db.prepare('select id, url, response_body, timestamp from http_cache order by id').all();

const categories = new Map();   // id -> category
const panels = new Map();       // panel response id -> {id, categories:Set}
const effects = new Map();      // effect_id -> effect
const typeCounts = new Map();

const num = v => (typeof v === 'string' && /^\d+$/.test(v) ? Number(v) : v);

for (const row of rows) {
  let body;
  try { body = JSON.parse(row.response_body); } catch { continue; }
  const data = body?.data;
  if (!data) continue;

  if (Array.isArray(data.categories)) {
    const panel = { id: row.id, source: data.panel_source ?? null, categories: [] };
    for (const c of data.categories) {
      const id = String(c.category_id);
      panel.categories.push(id);
      if (!categories.has(id)) {
        categories.set(id, {
          id: num(c.category_id), key: c.category_key ?? '', name: c.category_name ?? '',
          icon: c.category_icon || null, selectedIcon: c.category_selected_icon || null,
          parent: c.parent_category ?? null, vip: !!c.category_is_vip, top: !!c.category_is_top,
          panelIds: [], itemCount: 0, types: {},
        });
      }
      const cat = categories.get(id);
      if (!cat.panelIds.includes(row.id)) cat.panelIds.push(row.id);
    }
    panels.set(row.id, panel);
  }

  if (Array.isArray(data.effect_item_list)) {
    for (const it of data.effect_item_list) {
      const ca = it.common_attr ?? {};
      const eid = String(ca.effect_id ?? ca.id ?? '');
      if (!eid) continue;
      const t = ca.effect_type;
      typeCounts.set(t, (typeCounts.get(t) ?? 0) + 1);
      const cats = (ca.category_ids ?? []).map(String);
      for (const cid of cats) {
        const cat = categories.get(cid);
        if (cat) { cat.itemCount++; cat.types[t] = (cat.types[t] ?? 0) + 1; }
      }
      if (!effects.has(eid)) {
        let extra = null, sdkExtra = null, business = null;
        try { extra = ca.extra ? JSON.parse(ca.extra) : null; } catch { /* keep null */ }
        try { sdkExtra = ca.sdk_extra ? JSON.parse(ca.sdk_extra) : null; } catch { /* keep null */ }
        try { business = ca.business_info?.json_str ? JSON.parse(ca.business_info.json_str) : null; } catch { /* keep null */ }
        effects.set(eid, {
          id: eid,
          title: ca.title ?? ca.description ?? '',
          description: ca.description ?? '',
          type: t,
          subType: ca.sub_type ?? null,
          source: ca.source ?? null,
          md5: ca.md5 ?? null,
          packageUrl: (ca.item_urls ?? [])[0] ?? null,
          coverUrl: ca.cover_url?.small ?? ca.cover_url?.static_img ?? null,
          staticImage: ca.cover_url?.static_img ?? null,
          categories: cats,
          tags: (ca.tag_list ?? []).map(x => (typeof x === 'string' ? x : x?.name)).filter(Boolean),
          vip: !!business?.is_vip,
          aspectRatio: ca.aspect_ratio ?? null,
          itemLanguage: ca.item_language ?? null,
          publishSource: ca.publish_source ?? null,
          createTime: ca.create_time ?? null,
          extra, sdkExtra,
        });
      } else {
        const e = effects.get(eid);
        for (const c of cats) if (!e.categories.includes(c)) e.categories.push(c);
      }
    }
  }
}

const KIND_BY_TYPE = {
  12: 'filtros', 13: 'efectos', 48: 'texto-subtitulos', 97: 'mascaras',
  21: 'retoque', 121: 'retoque', 46: 'ia-imagen', 7: 'stickers', 58: 'stickers-personajes',
  26: 'audio-efectos', 47: 'edicion-automatica', 19: 'animaciones', 20: '3d-ajuste',
  69: 'encuadre', 9: 'fondos', 8: 'efectos',
};

function inferredKind(typeHistogram) {
  const entries = Object.entries(typeHistogram ?? {}).map(([t, n]) => [Number(t), n]).sort((a, b) => b[1] - a[1]);
  if (!entries.length) return { kind: 'sin-datos', confidence: 'none' };
  const [topType, topN] = entries[0];
  const total = entries.reduce((s, [, n]) => s + n, 0);
  const share = topN / total;
  return {
    kind: KIND_BY_TYPE[topType] ?? `tipo-${topType}`,
    confidence: share >= 0.7 ? 'alta' : share >= 0.4 ? 'media' : 'baja',
    dominantType: topType, share: Number(share.toFixed(2)),
  };
}

const panelOut = [...panels.values()].map(p => {
  const cats = p.categories.map(id => categories.get(id)).filter(Boolean);
  const hist = {};
  for (const c of cats) for (const [t, n] of Object.entries(c.types)) hist[t] = (hist[t] ?? 0) + n;
  return {
    panelId: String(p.id), source: p.source,
    ...inferredKind(hist),
    categoryCount: cats.length,
    itemCount: cats.reduce((s, c) => s + c.itemCount, 0),
    categories: cats.map(c => ({ id: c.id, key: c.key, name: c.name, items: c.itemCount })),
  };
}).sort((a, b) => b.itemCount - a.itemCount);

const catalog = {
  meta: {
    generatedAt: new Date().toISOString(),
    source: DB,
    sourceSha256: undefined,
    panels: panelOut.length,
    categories: categories.size,
    uniqueEffects: effects.size,
    cachedItems: [...typeCounts.values()].reduce((a, b) => a + b, 0),
    coverage: 'Solo lo que CapCut llego a pedir y guardar en su cache HTTP (paneles y categorias visitados). No es el catalogo completo del servidor.',
  },
  panels: panelOut,
  categories: [...categories.values()].map(c => ({
    id: c.id, key: c.key, name: c.name,
    panelIds: c.panelIds, itemCount: c.itemCount,
    types: c.types, kind: inferredKind(c.types).kind,
    icon: c.icon, vip: c.vip,
  })).sort((a, b) => b.itemCount - a.itemCount),
  effects: [...effects.values()],
};

fs.mkdirSync('catalog', { recursive: true });
fs.writeFileSync('catalog/capcut-catalog.json', JSON.stringify(catalog, null, 0));
console.log('panels:', panelOut.length, '| categories:', categories.size, '| unique effects:', effects.size);
console.log('\npanels inferred:');
for (const p of panelOut) {
  console.log(`${String(p.itemCount).padStart(6)} items  ${String(p.categoryCount).padStart(3)} cats  ${p.kind.padEnd(18)} conf=${p.confidence}  top=${p.dominantType ?? '-'}  -> ${p.categories.slice(0, 5).map(c => c.name).join(', ')}`);
}
const withItems = [...categories.values()].filter(c => c.itemCount > 0).length;
console.log(`\ncategories with cached items: ${withItems}/${categories.size}`);
