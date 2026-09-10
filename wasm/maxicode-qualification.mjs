import assert from "node:assert/strict";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { createRequire } from "node:module";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createHash } from "node:crypto";
const require = createRequire(
  resolve(process.env.MAXICODE_COMPONENTS_ROOT, "package.json"),
);
const { Miniflare } = createRequire(require.resolve("wrangler/package.json"))(
  "miniflare",
);
const pkg = fileURLToPath(new URL("./pkg/", import.meta.url));
const out = process.env.MAXICODE_OUTPUT_DIR;
mkdirSync(out, { recursive: true });
const font = readFileSync(process.env.MAXICODE_CJK_FONT);
assert.equal(
  createHash("sha256").update(font).digest("hex"),
  "c7763f454946833081cc90e73186615f8e1189de9c5e5a5a8752871fd79fddbc",
);
const worker = `import module from './zpl_forge_bg.wasm';
import * as glue from './zpl_forge_bg.js';
const instance = new WebAssembly.Instance(module, {'./zpl_forge_bg.js': glue});
glue.__wbg_set_wasm(instance.exports);
let font;
export default {async fetch(request) {
 if (new URL(request.url).pathname === '/font') {font = new Uint8Array(await request.arrayBuffer()); return new Response('ok');}
 const renderer = new glue.ZplRenderer();
 try {
  renderer.setDefaultFont(font);
  const {zpl,format,dpi} = await request.json();
  const result = renderer[format === 'png' ? 'renderPng' : 'renderShapedPdf'](zpl,Math.min(4*dpi,2000),Math.min(5*dpi,2000),dpi);
  return new Response(result,{headers:{'x-wasm-memory':String(instance.exports.memory.buffer.byteLength)}});
 } catch (error) {return new Response(error.message,{status:422});}
 finally {renderer.free();}
}};`;
const mf = new Miniflare({
  host: "127.0.0.1",
  port: 0,
  workers: [
    {
      config: {
        name: "maxicode-qualification",
        type: "worker",
        compatibilityDate: "2026-09-02",
        manifest: {
          mainModule: "index.js",
          modulesRoot: pkg,
          modules: {
            "index.js": { type: "esm", contents: worker },
            "zpl_forge_bg.js": {
              type: "esm",
              contents: readFileSync(resolve(pkg, "zpl_forge_bg.js"), "utf8"),
            },
            "zpl_forge_bg.wasm": {
              type: "wasm",
              contents: readFileSync(resolve(pkg, "zpl_forge_bg.wasm")),
            },
          },
        },
      },
    },
  ],
});
const report = [];
try {
  assert.equal(
    (
      await mf.dispatchFetch("http://qualification.local/font", {
        method: "POST",
        body: font,
      })
    ).status,
    200,
  );
  for (const [mode, data] of [
    [2, "002840336091062[)>_1E01_1D961Z12345678_1DUPSN_1E_04"],
    [3, "001124K1A0B1[)>_1E01_1D961Z12345678_1DUPSN_1E_04"],
    [4, "HELLO MAXICODE 123456789"],
  ]) {
    for (const dpi of [203, 300, 600]) {
      const zpl = `^XA^CI28^FO30,20^A0N,30,30^FD中文箱标 Shipping Label^FS^FO30,90^BY6^BD${mode}^FH^FD${data}^FS^FO30,740^BY2^BCN,70,N,N,N^FD123456789012^FS^XZ`;
      for (const format of ["pdf", "png"]) {
        const response = await mf.dispatchFetch(
          "http://qualification.local/render",
          { method: "POST", body: JSON.stringify({ zpl, format, dpi }) },
        );
        assert.equal(
          response.status,
          200,
          response.status === 200 ? undefined : await response.text(),
        );
        const bytes = Buffer.from(await response.arrayBuffer());
        writeFileSync(resolve(out, `mode${mode}-${dpi}.${format}`), bytes);
        report.push({
          mode,
          dpi,
          format,
          bytes: bytes.length,
          returnedLinearMemory: Number(response.headers.get("x-wasm-memory")),
        });
      }
    }
  }
  for (const command of ["^BD5", "^BD4,2,2", "^BDgarbage"]) {
    const response = await mf.dispatchFetch(
      "http://qualification.local/render",
      {
        method: "POST",
        body: JSON.stringify({
          zpl: `^XA^FO30,90${command}^FDHELLO^FS^XZ`,
          format: "pdf",
          dpi: 203,
        }),
      },
    );
    assert.equal(response.status, 422);
  }
  writeFileSync(
    resolve(out, "workerd-results.json"),
    JSON.stringify(report, null, 2) + "\n",
  );
  console.log(
    `workerd: ${report.length} renders passed; unsupported mode/sequence/malformed command rejected`,
  );
} finally {
  await mf.dispose();
}
