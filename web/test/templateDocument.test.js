import assert from "node:assert/strict";
import { test } from "node:test";
import {
  collidingRuleTags,
  definedRuleTags,
  displayPattern,
  finalRuleTag,
  isRuleTagSelected,
  matchNodeTags,
  migrateLegacyRuleSets,
  normalizeTemplateContent,
  referencedRuleTags,
  removeRuleSourceTag,
  toggleRuleSourceTag,
  writeNodeGroupPattern,
} from "../src/data/templateDocument.js";

test("normalize adds direct and block, drops root outbounds, and is idempotent", () => {
  const input = {
    outbounds: [{ tag: "old", type: "direct" }],
    policy_groups: [{ tag: "默认策略", type: "selector", outbounds: ["直连"] }],
    dns: { servers: [{ tag: "dns_direct" }] },
  };
  const once = normalizeTemplateContent(input);
  const twice = normalizeTemplateContent(once);

  assert.deepEqual(once.policy_groups[0], { tag: "直连", type: "direct" });
  assert.deepEqual(once.policy_groups[1], input.policy_groups[0]);
  assert.deepEqual(once.policy_groups[2], { tag: "block", type: "block" });
  assert.equal(Object.hasOwn(once, "outbounds"), false);
  assert.deepEqual(once.dns, input.dns);
  assert.deepEqual(twice, once);
  assert.equal(JSON.stringify(twice), JSON.stringify(once));
  assert.deepEqual(input.outbounds, [{ tag: "old", type: "direct" }]);
});

test("normalize keeps an existing direct tag and does not rewrite placeholders", () => {
  const input = {
    policy_groups: [{ tag: "direct", type: "direct" }],
    node_groups: [
      { tag: "香港节点", type: "urltest", outbounds: ["{(?i)(港|hk)}", "{.*}"] },
    ],
    rule_sets: [{ tag: ["cn"] }],
  };
  const next = normalizeTemplateContent(input);

  assert.equal(next.policy_groups.filter((group) => group.type === "direct").length, 1);
  assert.equal(next.policy_groups[0].tag, "direct");
  assert.equal(next.policy_groups.at(-1).type, "block");
  assert.deepEqual(next.node_groups[0].outbounds, ["{(?i)(港|hk)}", "{.*}"]);
  assert.deepEqual(next.rule_sets, input.rule_sets);
});

test("display reads only the first placeholder and hides a leading flag", () => {
  assert.equal(displayPattern(["{(?i)(港|hk)}", "{.*}"]), "(港|hk)");
  assert.equal(displayPattern(["{.*}"]), ".*");
  assert.equal(displayPattern(["{My-}"]), "My-");
  assert.equal(displayPattern([]), "");
  assert.equal(displayPattern(undefined), "");
});

test("writing a pattern collapses to one entry and does not store (?i)", () => {
  const input = {
    node_groups: [
      { tag: "香港节点", type: "urltest", outbounds: ["{(?i)(港|hk)}", "{.*}"] },
      { tag: "ALL", type: "urltest", outbounds: ["{.*}"] },
    ],
  };
  const next = writeNodeGroupPattern(input, 0, "(?i)(港|hk)");

  assert.deepEqual(next.node_groups[0].outbounds, ["{(港|hk)}"]);
  assert.deepEqual(next.node_groups[1].outbounds, ["{.*}"]);
  assert.deepEqual(input.node_groups[0].outbounds, ["{(?i)(港|hk)}", "{.*}"]);
  assert.deepEqual(writeNodeGroupPattern(input, 1, ".*").node_groups[1].outbounds, ["{.*}"]);
  assert.deepEqual(writeNodeGroupPattern(input, 0, "  ").node_groups[0].outbounds, []);
  assert.deepEqual(writeNodeGroupPattern(input, 0, "{My-}").node_groups[0].outbounds, ["{My-}"]);
});

test("invalid patterns have zero hits and duplicate tags keep the first one", () => {
  assert.deepEqual(matchNodeTags("hk", ["HK-1", "jp", "HK-1"]), {
    ok: true,
    tags: ["HK-1"],
  });
  assert.deepEqual(matchNodeTags("{(?i)hk}", ["HK"]), { ok: true, tags: ["HK"] });
  assert.deepEqual(matchNodeTags(".*", ["a", "b", "a"]), { ok: true, tags: ["a", "b"] });
  assert.deepEqual(matchNodeTags("", ["a"]), { ok: true, tags: [] });
  assert.deepEqual(matchNodeTags("(?", ["foo(", "a"]), { ok: false, tags: [] });
});

test("final rule tags ignore a bare stored name when a prefix is set", () => {
  const source = { tag_prefix: "geosite-", tag: "cn" };
  assert.equal(finalRuleTag("geosite-", "cn"), "geosite-cn");
  assert.equal(finalRuleTag("geosite-", "geosite-cn"), "geosite-cn");
  assert.equal(finalRuleTag("", "cn"), "cn");
  assert.equal(isRuleTagSelected(source, "cn"), false);
  assert.equal(isRuleTagSelected({ tag_prefix: "geosite-", tag: ["geosite-cn"] }, "cn"), true);
});

test("toggle writes the final tag and does not touch route rules", () => {
  const input = {
    rule_sets: [{ tag_prefix: "geosite-", tag: "cn" }],
    route: { rules: [{ rule_set: ["cn"], outbound: "直连" }] },
  };
  const added = toggleRuleSourceTag(input, 0, "cn");
  assert.deepEqual(added.rule_sets[0].tag, ["cn", "geosite-cn"]);
  assert.deepEqual(added.route, input.route);
  const removed = toggleRuleSourceTag(added, 0, "cn");
  assert.deepEqual(removed.rule_sets[0].tag, ["cn"]);
  assert.deepEqual(removeRuleSourceTag(added, 0, "cn").rule_sets[0].tag, ["geosite-cn"]);
});

test("collisions count a second copy and references stay on the outer rule", () => {
  const content = {
    rule_sets: [
      { tag: ["geosite-cn", "geosite-cn"] },
      { tag: "geoip-cn" },
    ],
    route: {
      rules: [
        { rule_set: ["geosite-cn", "missing"], outbound: "直连" },
        { type: "logical", mode: "and", rules: [{ rule_set: ["nested"] }] },
      ],
    },
    dns: { rules: [{ rule_set: "geoip-cn", server: "dns_direct" }] },
  };
  assert.deepEqual(collidingRuleTags(content), ["geosite-cn"]);
  assert.deepEqual(referencedRuleTags(content), ["geosite-cn", "missing", "geoip-cn"]);
  assert.deepEqual(definedRuleTags(content), ["geosite-cn", "geoip-cn"]);
});

test("legacy route rule_set folds into one DustinWin source and stays idempotent", () => {
  const input = {
    route: {
      final: "直连",
      rule_set: [{ tag: "cn" }, { tag: ["ai", "cn", ""] }],
      rules: [],
    },
  };
  const once = migrateLegacyRuleSets(input);
  const twice = migrateLegacyRuleSets(once);
  assert.equal(once.rule_sets.length, 1);
  assert.equal(once.rule_sets[0].preset_id, "dustinwin-ruleset");
  assert.equal(once.rule_sets[0].tag_prefix, "");
  assert.deepEqual(once.rule_sets[0].tag, ["cn", "ai"]);
  assert.equal(once.route.rule_set, undefined);
  assert.equal(once.route.final, "直连");
  assert.equal(migrateLegacyRuleSets({ rule_sets: [{ tag: ["kept"] }], route: { rule_set: [{ tag: "cn" }] } }).rule_sets[0].tag[0], "kept");
  assert.equal(JSON.stringify(twice), JSON.stringify(once));
  assert.deepEqual(migrateLegacyRuleSets({ dns: {} }), { dns: {} });
  assert.deepEqual(definedRuleTags(input), ["cn", "ai"]);
});
