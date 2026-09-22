import defaultTemplateRaw from './defaultTemplate.json';

export const initialTemplates = [
  {
    id: "tpl_tailscale_gateway",
    name: "Tailscale & 分流网关模板",
    description: "生产级透明代理模板：包含完整的 DNS、Inbounds/Tailscale 端点、分流策略组与 18 条路由规则",
    updatedAt: "2026-09-16 12:30",
    content: JSON.parse(JSON.stringify(defaultTemplateRaw))
  },
  {
    id: "tpl_minimal_tun",
    name: "轻量移动端 TUN 模板",
    description: "专为 iOS / Android / macOS 客户端准备的精简 TUN 模式模板",
    updatedAt: "2026-09-15 09:15",
    content: {
      log: { level: "warn", timestamp: true },
      dns: {
        servers: [
          { tag: "dns_proxy", type: "https", server: "1.1.1.1", detour: "默认策略" },
          { tag: "dns_direct", type: "https", server: "223.5.5.5" }
        ],
        rules: [
          { outbound: "any", server: "dns_direct" }
        ]
      },
      inbounds: [
        { type: "tun", tag: "tun-in", interface_name: "utun", inet4_address: "172.19.0.1/30", auto_route: true, strict_route: true }
      ],
      policy_groups: [
        { type: "selector", tag: "默认策略", outbounds: ["全部节点", "直连"] }
      ],
      node_groups: [
        { type: "urltest", tag: "全部节点", outbounds: ["{.*}"] }
      ],
      outbounds: [
        { type: "direct", tag: "直连" }
      ],
      route: {
        auto_detect_interface: true,
        rules: [
          { protocol: "dns", outbound: "dns-out" },
          { ip_is_private: true, outbound: "直连" },
          { rule_set: "cn", outbound: "直连" }
        ],
        rule_set: [
          { tag: "cn", type: "remote", format: "binary", url: "https://raw.githubusercontent.com/SagerNet/sing-geosite/rule-set/geosite-cn.srs" }
        ]
      }
    }
  }
];

export const initialSources = [
  {
    id: "src_airport_main",
    name: "主力机场订阅 (Airport Subscription)",
    type: "subscription",
    url: "https://sub.fastspeed.xyz/api/v1/client/subscribe?token=airport_demo_98231",
    lastUpdated: "2026-09-16 14:10",
    status: "active",
    nodeCount: 8,
    nodes: [
      {
        type: "vmess",
        tag: "香港 01 [BGP 高速]",
        server: "hk01.fastspeed.xyz",
        server_port: 443,
        uuid: "3b2c1a0d-9e8f-4c5b-8a7d-6e5f4a3b2c1d",
        security: "auto",
        alter_id: 0,
        transport: { type: "ws", path: "/vmess-ws" },
        tls: { enabled: true, server_name: "hk01.fastspeed.xyz" }
      },
      {
        type: "vless",
        tag: "香港 02 [IEPL 专线]",
        server: "hk02.fastspeed.xyz",
        server_port: 443,
        uuid: "4c3d2e1f-0a9b-8c7d-6e5f-4a3b2c1d0e9f",
        flow: "xtls-rprx-vision",
        tls: { enabled: true, server_name: "hk02.fastspeed.xyz", reality: { enabled: true, public_key: "AbCdEf0123456789" } }
      },
      {
        type: "trojan",
        tag: "台湾 01 [动态家宽]",
        server: "tw01.fastspeed.xyz",
        server_port: 443,
        password: "TrojanPassword123",
        tls: { enabled: true, server_name: "tw01.fastspeed.xyz" }
      },
      {
        type: "hysteria2",
        tag: "日本 01 [软银 10G]",
        server: "jp01.fastspeed.xyz",
        server_port: 8443,
        password: "Hy2SecretTokyo99",
        tls: { enabled: true, server_name: "jp01.fastspeed.xyz" }
      },
      {
        type: "shadowsocks",
        tag: "日本 02 [原生 IP 流媒体]",
        server: "jp02.fastspeed.xyz",
        server_port: 8388,
        method: "2022-blake3-aes-128-gcm",
        password: "ss2022password=="
      },
      {
        type: "vmess",
        tag: "新加坡 01 [BGP 首尔中转]",
        server: "sg01.fastspeed.xyz",
        server_port: 443,
        uuid: "5d4e3f2a-1b0c-9d8e-7f6a-5b4c3d2e1f0a",
        security: "auto",
        transport: { type: "ws", path: "/sg" },
        tls: { enabled: true, server_name: "sg01.fastspeed.xyz" }
      },
      {
        type: "vless",
        tag: "美国 01 [Anycast 优化]",
        server: "us01.fastspeed.xyz",
        server_port: 443,
        uuid: "6e5f4a3b-2c1d-0e9f-8a7b-6c5d4e3f2a1b",
        tls: { enabled: true, server_name: "us01.fastspeed.xyz" }
      },
      {
        type: "shadowsocks",
        tag: "美国 02 [原生广播 IP]",
        server: "us02.fastspeed.xyz",
        server_port: 8388,
        method: "aes-256-gcm",
        password: "standard_ss_pwd"
      }
    ]
  },
  {
    id: "src_private_vps",
    name: "自建与私有节点 (Private VPS Nodes)",
    type: "manual",
    url: "",
    lastUpdated: "2026-09-15 18:00",
    status: "active",
    nodeCount: 3,
    nodes: [
      {
        type: "hysteria2",
        tag: "My-Tokyo-Direct",
        server: "198.51.100.22",
        server_port: 30443,
        password: "MyTokyoPassword999",
        tls: { enabled: true, server_name: "tokyo.myvps.net", insecure: true }
      },
      {
        type: "vless",
        tag: "World-US-01",
        server: "203.0.113.88",
        server_port: 443,
        uuid: "7f6a5b4c-3d2e-1f0a-9b8c-7d6e5f4a3b2c",
        tls: { enabled: true, server_name: "us.myworld.org", reality: { enabled: true, public_key: "PublicKeyReality99" } }
      },
      {
        type: "shadowsocks",
        tag: "良心云-HK-Transit",
        server: "129.226.15.66",
        server_port: 58388,
        method: "2022-blake3-aes-128-gcm",
        password: "TencentCloudHk=="
      }
    ]
  }
];

export const initialProfiles = [
  {
    id: "prof_home_router",
    name: "家庭 OpenWrt 生产网关",
    description: "路由器透明代理主配置，包含所有机场节点与自建高速节点，支持 Tailscale 互联",
    templateId: "tpl_tailscale_gateway",
    sourceIds: ["src_airport_main", "src_private_vps"],
    token: "tok_router_prod_e8a91f4b",
    publicUrl: "http://studio.internal.lan:8080/sub/tok_router_prod_e8a91f4b",
    updatedAt: "2026-09-16 14:15"
  },
  {
    id: "prof_mobile_travel",
    name: "移动端/笔记本轻量配置",
    description: "手机与笔记本使用的轻量 TUN 模式，仅加载机场高质量节点",
    templateId: "tpl_minimal_tun",
    sourceIds: ["src_airport_main"],
    token: "tok_mobile_travel_3c7d0a12",
    publicUrl: "http://studio.internal.lan:8080/sub/tok_mobile_travel_3c7d0a12",
    updatedAt: "2026-09-16 10:05"
  }
];

export function testRegexMatch(pattern, tag) {
  let flags = 'i';
  let cleanPattern = (pattern || '').trim();
  if (cleanPattern.startsWith('{') && cleanPattern.endsWith('}') && cleanPattern.length >= 2) {
    cleanPattern = cleanPattern.slice(1, -1).trim();
  }
  if (cleanPattern.startsWith('(?i)')) {
    cleanPattern = cleanPattern.slice(4).trim();
  }
  try {
    const re = new RegExp(cleanPattern, flags);
    return re.test(tag);
  } catch (e) {
    return tag.toLowerCase().includes(cleanPattern.toLowerCase());
  }
}

export function compileProfile(template, boundSources) {
  if (!template || !template.content) {
    return { config: {}, matchedMap: {}, totalNodes: 0 };
  }

  // 1. Gather all nodes from bound sources
  const allNodes = [];
  const seenTags = new Set();
  for (const src of boundSources) {
    if (src.nodes && Array.isArray(src.nodes)) {
      for (const node of src.nodes) {
        if (!seenTags.has(node.tag)) {
          seenTags.add(node.tag);
          allNodes.push(JSON.parse(JSON.stringify(node)));
        }
      }
    }
  }

  // 2. Clone template content
  const compiled = JSON.parse(JSON.stringify(template.content));
  const matchedMap = {}; // groupTag -> list of matched node tags
  const usedNodeTags = new Set();

  const policy_groups = Array.isArray(compiled.policy_groups) ? compiled.policy_groups : [];
  const node_groups = Array.isArray(compiled.node_groups) ? compiled.node_groups : [];
  const base_outbounds = Array.isArray(compiled.outbounds) ? compiled.outbounds : [];

  const fallbackTag = base_outbounds.find(o => o.type === 'direct')?.tag || 'direct';

  const expanded_node_groups = [];
  for (const group of node_groups) {
    if (Array.isArray(group.outbounds)) {
      const newTargets = [];
      matchedMap[group.tag] = [];

      for (const target of group.outbounds) {
        if (typeof target === 'string' && target.startsWith('{') && target.endsWith('}')) {
          const pattern = target.slice(1, -1);
          const matchedTags = allNodes
            .filter(n => testRegexMatch(pattern, n.tag))
            .map(n => n.tag);

          if (matchedTags.length > 0) {
            for (const mt of matchedTags) {
              if (!newTargets.includes(mt)) {
                newTargets.push(mt);
                matchedMap[group.tag].push(mt);
                usedNodeTags.add(mt);
              }
            }
          }
        } else {
          newTargets.push(target);
          if (allNodes.some(n => n.tag === target)) {
            usedNodeTags.add(target);
          }
        }
      }

      if (newTargets.length === 0) {
        newTargets.push(fallbackTag);
      }

      const g = JSON.parse(JSON.stringify(group));
      g.outbounds = newTargets;
      expanded_node_groups.push(g);
    }
  }

  // 3. Assemble root outbounds: policy_groups + expanded_node_groups + base_outbounds + matched nodes
  const assembledOutbounds = [
    ...policy_groups,
    ...expanded_node_groups,
    ...base_outbounds
  ];

  const existingTags = new Set(assembledOutbounds.map(o => o.tag).filter(Boolean));
  const nodesToAppend = allNodes.filter(n => usedNodeTags.has(n.tag));
  for (const node of nodesToAppend) {
    if (!existingTags.has(node.tag)) {
      assembledOutbounds.push(node);
      existingTags.add(node.tag);
    }
  }

  delete compiled.policy_groups;
  delete compiled.node_groups;
  compiled.outbounds = assembledOutbounds;

  return {
    config: compiled,
    matchedMap,
    totalNodes: allNodes.length,
    usedCount: usedNodeTags.size
  };
}
