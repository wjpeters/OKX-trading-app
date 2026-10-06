import test from "node:test";
import assert from "node:assert/strict";
import {
  draftDrawing,
  parseDrawings,
  targetPrice,
  translateDrawing,
  indexAt,
  timeAt,
  linePrice,
  drawingTools,
} from "../src/drawing-model.ts";
const a = { time: 1000, price: 100 },
  b = { time: 3000, price: 120 };
const candles = [{ ts: 1000 }, { ts: 2000 }, { ts: 4000 }];
test("all nine tools have explicit placement steps", () => {
  assert.equal(drawingTools.filter((t) => t.kind !== "select").length, 9);
  for (const t of drawingTools.filter((t) => t.kind !== "select"))
    assert.ok(t.steps.length > 0);
});
test("channel width follows the third point on either side of the slope", () => {
  const d = draftDrawing(
    "channel",
    [a, b],
    { time: 2000, price: 130 },
    "#89bfc1",
  );
  assert.equal(d.offset, -20);
  assert.equal(linePrice(a, b, 2000) - d.offset, 130);
  assert.equal(
    draftDrawing("channel", [a, b], { time: 2000, price: 90 }, "#89bfc1")
      .offset,
    20,
  );
});
test("long preview defaults to 2:1 and respects a chosen target", () => {
  const stop = { time: 3000, price: 90 };
  assert.equal(targetPrice(draftDrawing("risk", [a], stop, "#89bfc1")), 120);
  assert.equal(
    targetPrice(
      draftDrawing("risk", [a, stop], { time: 4000, price: 135 }, "#89bfc1"),
    ),
    135,
  );
});
test("short preview projects reward below entry", () => {
  assert.equal(targetPrice(draftDrawing("risk", [a], b, "#89bfc1")), 60);
});
test("moving a risk drawing preserves risk/reward distances", () => {
  const d = draftDrawing("risk", [a, b], { time: 4000, price: 60 }, "#89bfc1");
  const moved = translateDrawing(d, 9000, 30);
  assert.deepEqual(moved.a, { time: 10000, price: 130 });
  assert.deepEqual(moved.b, { time: 12000, price: 150 });
  assert.equal(moved.target, 90);
  assert.equal(moved.target - moved.a.price, d.target - d.a.price);
});
test("moving a channel preserves its signed offset", () => {
  const d = draftDrawing(
    "channel",
    [a, b],
    { time: 2000, price: 130 },
    "#89bfc1",
  );
  assert.equal(translateDrawing(d, 100, 15).offset, -20);
});
test("timestamp projection round trips gaps, fractional bars and future space", () => {
  for (const index of [-5, 0, 0.5, 1, 1.5, 2, 7])
    assert.ok(
      Math.abs(indexAt(candles, timeAt(candles, index)) - index) < 1e-10,
    );
});
test("single-candle projection remains finite", () => {
  assert.equal(indexAt([{ ts: 1000 }], timeAt([{ ts: 1000 }], 2)), 2);
});
test("legacy v1 drawings keep IDs, locks, text and old risk target", () => {
  const [d] = parseDrawings(
    JSON.stringify([
      { id: "saved", kind: "risk", a, b, locked: true, text: "oud" },
    ]),
  );
  assert.equal(d.id, "saved");
  assert.equal(d.locked, true);
  assert.equal(d.text, "oud");
  assert.equal(targetPrice(d), 60);
});
test("malformed saved drawings fail without silently overwriting the source", () => {
  for (const raw of [
    "{",
    "{}",
    '[{"kind":"trend"}]',
    JSON.stringify([{ id: "bad", kind: "select", a, b, text: "" }]),
  ])
    assert.throws(() => parseDrawings(raw));
  assert.deepEqual(parseDrawings(null), []);
});

test("future monthly drawing anchors follow leap years and quarter boundaries", () => {
  const cs = [{ ts: Date.UTC(2024, 0, 1) }, { ts: Date.UTC(2024, 1, 1) }];
  assert.equal(timeAt(cs, 2, "1Mutc"), Date.UTC(2024, 2, 1));
  assert.equal(timeAt(cs, 3, "1Mutc"), Date.UTC(2024, 3, 1));
  for (const i of [-2, -0.5, 0, 0.5, 1, 2, 2.5, 13])
    assert.ok(
      Math.abs(indexAt(cs, timeAt(cs, i, "1Mutc"), "1Mutc") - i) < 1e-10,
    );
  const qs = [{ ts: Date.UTC(2025, 6, 1) }, { ts: Date.UTC(2025, 9, 1) }];
  assert.equal(timeAt(qs, 2, "3Mutc"), Date.UTC(2026, 0, 1));
  assert.equal(indexAt(qs, Date.UTC(2026, 3, 1), "3Mutc"), 3);
});
