#!/usr/bin/env python3
import json, os, time, urllib.request, urllib.error
from datetime import datetime

BASE_URL = "http://localhost:8003"
OUTPUT_DIR = "benchmark-results/real-memory-benchmark"

def get_required_xavier_token():
    for env_var in ("XAVIER_TOKEN", "XAVIER_API_KEY", "XAVIER_TOKEN"):
        token = os.environ.get(env_var, "").strip()
        if token:
            return token
    raise RuntimeError("Missing Xavier token. Set XAVIER_TOKEN, XAVIER_API_KEY, or XAVIER_TOKEN.")

TOKEN = get_required_xavier_token()

def api(path, payload=None, method="POST"):
    url = BASE_URL + path
    data = json.dumps(payload).encode("utf-8") if payload else None
    req = urllib.request.Request(url, data=data, method=method,
        headers={"Content-Type": "application/json", "X-Xavier-Token": TOKEN})
    try:
        with urllib.request.urlopen(req, timeout=60) as r:
            return json.loads(r.read().decode("utf-8"))
    except urllib.error.HTTPError as e:
        if e.code == 405 and method == "GET":
            req = urllib.request.Request(url, method="GET",
                headers={"X-Xavier-Token": TOKEN})
            with urllib.request.urlopen(req, timeout=60) as r:
                return json.loads(r.read().decode("utf-8"))
        raise

def wait_health():
    for _ in range(60):
        try:
            with urllib.request.urlopen(BASE_URL + "/health", timeout=5) as r:
                if r.status == 200: return
        except: time.sleep(1)
    raise RuntimeError("Xavier not healthy")

def load_docs(docs):
    api("/memory/reset", {})
    t0 = time.time()
    for d in docs:
        api("/memory/add", {"path":d["path"],"content":d["content"],"metadata":d.get("metadata",{}),
            "kind":d.get("kind"),"evidence_kind":d.get("evidence_kind"),
            "namespace":d.get("namespace"),"provenance":d.get("provenance")})
    return (time.time()-t0)/len(docs)*1000

def do_search(query, filters=None, limit=5):
    p = {"query":query,"limit":limit}
    if filters: p["filters"] = filters
    t0 = time.time()
    r = api("/memory/search", p)
    return r.get("results",[]), (time.time()-t0)*1000

def do_query(query, filters=None, system3="disabled"):
    p = {"query":query,"limit":5}
    if filters: p["filters"] = filters
    p["system3_mode"] = system3
    t0 = time.time()
    r = api("/memory/query", p)
    return r.get("response",""), (time.time()-t0)*1000

def do_agents(query, filters=None):
    p = {"query":query,"limit":5}
    if filters: p["filters"] = filters
    t0 = time.time()
    r = api("/agents/run", p)
    return r.get("response",""), (time.time()-t0)*1000

print("Waiting for Xavier...")
wait_health()
print("Xavier is healthy.")

with open("E:\\scripts-python\\xavier\\scripts\\benchmarks\\datasets\\internal_swal_openclaw_memory.json", encoding="utf-8") as f:
    dataset = json.load(f)

docs = dataset["documents"]
cases = dataset["cases"]

print(f"Loading {len(docs)} docs...")
load_ms = load_docs(docs)
print(f"Load time: {load_ms:.1f}ms/doc")

results = []
s_lat, q_lat = [], []
correct = 0

for case in cases:
    cid = case["id"]
    ep = case["endpoint"]
    q = case["query"]
    filt = case.get("filters")
    hit = False

    if ep == "search":
        res, ms = do_search(q, filt)
        s_lat.append(ms)
        top = res[0].get("path") if res else None
        exp = case.get("expected_path")
        hit = top == exp
        results.append({"id":cid,"endpoint":ep,"query":q,"expected":exp,"actual":top,"hit":hit,"latency_ms":round(ms,1)})
    elif ep == "query":
        resp, ms = do_query(q, filt, case.get("system3_mode","disabled"))
        q_lat.append(ms)
        exp = case.get("expected_substring","")
        hit = exp.lower() in resp.lower()
        results.append({"id":cid,"endpoint":ep,"query":q,"expected":exp,"actual":resp[:200],"hit":hit,"latency_ms":round(ms,1)})
    elif ep == "agents_run":
        resp, ms = do_agents(q, filt)
        q_lat.append(ms)
        exp = case.get("expected_substring","")
        hit = exp.lower() in resp.lower()
        results.append({"id":cid,"endpoint":ep,"query":q,"expected":exp,"actual":resp[:200],"hit":hit,"latency_ms":round(ms,1)})

    correct += 1 if hit else 0
    print(f"  {'[PASS]' if hit else '[FAIL]'} {cid} ({results[-1]['latency_ms']:.0f}ms)")

acc = correct/len(cases)*100
avg_s = sum(s_lat)/len(s_lat) if s_lat else 0
avg_q = sum(q_lat)/len(q_lat) if q_lat else 0
build = api("/build", {}, "GET")
summary = {"timestamp":datetime.now().isoformat(),"backend":build["memory_store"]["backend"],
    "version":build["version"],"total_cases":len(cases),"passed":correct,
    "accuracy_pct":round(acc,1),"avg_search_ms":round(avg_s,1),"avg_query_ms":round(avg_q,1),
    "load_ms_per_doc":round(load_ms,1),"search_cases":len(s_lat),"query_cases":len(q_lat)}

print(f"\n{'='*50}")
print(f"RESULTS -- {summary['backend']} backend (v{summary['version']})")
print(f"{'='*50}")
print(f"Accuracy:    {acc:.1f}%  ({correct}/{len(cases)})")
print(f"Avg search:  {avg_s:.1f}ms")
print(f"Avg query:   {avg_q:.1f}ms")
print(f"Load/doc:    {load_ms:.1f}ms")

import os
os.makedirs(OUTPUT_DIR, exist_ok=True)
with open(f"{OUTPUT_DIR}/summary.json","w",encoding="utf-8") as f: json.dump(summary,f,indent=2)
with open(f"{OUTPUT_DIR}/records.json","w",encoding="utf-8") as f: json.dump(results,f,indent=2)
print(f"Saved: {OUTPUT_DIR}/summary.json and records.json")
