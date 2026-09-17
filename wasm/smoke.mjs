import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import * as glue from './pkg/zpl_forge_bg.js'

const bytes = readFileSync(new URL('./pkg/zpl_forge_bg.wasm', import.meta.url))
const { instance } = await WebAssembly.instantiate(bytes, { './zpl_forge_bg.js': glue })
glue.__wbg_set_wasm(instance.exports)
const renderer = new glue.ZplRenderer()
const text = '^XA^FO30,30^A0N,40,40^FDWASM SMOKE^FS^XZ'
try {
  assert.equal(new TextDecoder().decode(renderer.renderPdf(text, 816, 1216, 203).slice(0, 5)), '%PDF-')
  assert.deepEqual([...renderer.renderPng(text, 816, 1216, 203).slice(0, 8)], [137,80,78,71,13,10,26,10])
  assert.throws(() => renderer.renderPdf(text, 0, 1216, 203), /ZPL_CANVAS_LIMIT/)
  assert.throws(() => renderer.setDefaultFont(new Uint8Array([1,2,3])), /FONT_INVALID/)
  assert.throws(() => renderer.renderPdf('^XA^FO30,30^A0N,40,40^FD中文^FS^XZ', 816, 1216, 203), /ZPL_FONT_UNSUPPORTED/)
  assert.throws(() => renderer.renderPdfPages(text, 816, 1216, 203, '[]'), /ZPL_PAGE_LIMIT/)
  assert.match(renderer.renderBarcodeSvg('QR', 'https://yqn.com', 4, 4), /^<svg /)
  assert.deepEqual([...renderer.renderBarcodePng('CODE_128', 'ABC-123', 2, 10).slice(0, 8)], [137,80,78,71,13,10,26,10])
  assert.throws(() => renderer.renderBarcodeSvg('UNKNOWN', 'x', 1, 0), /BARCODE_RENDER_FAILED/)
  console.log('WASM label and standalone barcode smoke passed')
} finally { renderer.free() }
