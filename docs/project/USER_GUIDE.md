# 📖 Atropos User & Use Case Guide

Atropos is a **Resource Orchestrator**. It ensures that when multiple people (or scripts) want the same limited thing at the exact same time, **only one person gets it**, and it is automatically taken back when they are done.

---

## 🛑 The Problem: "The Double-Booking Disaster"

Imagine you have **one** expensive NVIDIA H100 GPU for AI training. 
1. **Script A** checks if the GPU is free. The database says "Yes."
2. **Script B** checks if the GPU is free *at the exact same millisecond*. The database also says "Yes."
3. **Both scripts** start their AI jobs.
4. **Result:** The GPU crashes, the OS freezes, or both jobs fail after 5 hours of wasted electricity.

**Atropos solves this.** It uses "Atomic Locking" at the database level. If 1,000 scripts ask for 1 GPU at the same time, Atropos guarantees that 1 script gets a "Success" and 999 get a "Try again later" (or are put on a waitlist).

---

## 💡 Real-World Use Cases

### 1. AI & Machine Learning GPU Clusters
*   **The Resource:** A pool of 8x A100 GPUs.
*   **The Problem:** Data scientists launching Jupyter notebooks simultaneously, causing "Out of Memory" (OOM) errors.
*   **The Atropos Solution:** Users request a `v100-gpu` lease for 2 hours. Atropos gives them a specific GPU ID. After 2 hours, Atropos marks it as "Free" automatically, even if the user forgets to close their notebook.

### 2. CI/CD "Clean Room" Environments
*   **The Resource:** 5 dedicated physical macOS servers for iOS builds.
*   **The Problem:** Multiple GitHub Actions trying to run on the same Mac at once, corrupting the build cache.
*   **The Atropos Solution:** The CI pipeline calls Atropos: *"Give me a Mac for 15 minutes."* Atropos locks Mac #3 for that specific build. No other build can touch Mac #3 until the timer expires.

### 3. High-Value Software Licenses
*   **The Resource:** 10 floating licenses for a $50k/year CAD software.
*   **The Problem:** Using more licenses than you own leads to legal audits and massive fines.
*   **The Atropos Solution:** The software "checks out" a lease from Atropos on startup. If 10 are out, the 11th user is told to wait.

---

## 🛠 Preconditions (What you need before starting)

1.  **A PostgreSQL Database:** Atropos uses Postgres as its "Brain." It must be version 12+ to support the advanced locking.
2.  **Defined Pools:** You must tell Atropos what you are managing (e.g., "We have a pool called 'GPUs' with 10 items").
3.  **Network Access:** Atropos is a REST API. Your scripts or apps need to be able to send HTTP requests to it.

---

## 🚀 How to use Atropos (The Lifecycle)

### Step 1: Define your Pool
Tell Atropos you have a new type of resource.
```bash
# Example: Creating a pool for "Mac-Minis"
POST /pools
{
  "name": "iOS Build Farm",
  "resource_type": "mac-mini",
  "policy": "FIFO"
}
```

### Step 2: Add Resources
Add the actual items to that pool.
```bash
# Adding Mac #1 to the pool
POST /resources
{
  "pool_id": "UUID-OF-POOL",
  "external_id": "mac-mini-01",
  "attributes": { "os": "Sonoma", "ram": "32GB" }
}
```

### Step 3: Request a Lease (The "Magic" Part)
Your script asks for a resource. You can now specify priorities, constraints, and distribution rules.
```bash
POST /leases
{
  "pool_type": "mac-mini",
  "owner_id": "jenkins-job-42",
  "tenant_id": "ios-team",
  "priority": 100,
  "ttl_seconds": 600,
  "preempt": true,
  "constraints": { "ram": "32GB" },
  "spread_by": "rack_id"
}
```
**Response:**
*   `201 Created`: You got it! Here is your Resource ID.
*   `409 Conflict`: Sorry, they are all busy (and you couldn't preempt anyone).
*   `202 Accepted`: (If waitlist enabled) You are in line. Your priority will "age" up the longer you wait.

### Step 4: Keep it Alive (Heartbeating)
For long-running jobs, you should "heartbeat" your lease to prove your process hasn't crashed. If a heartbeat is missed for 60 seconds, Atropos may reclaim the resource.
```bash
POST /leases/:id/heartbeat
```

---

## 🛡 Reliability Guarantees
*   **Zero Double-Booking:** Guaranteed atomic claims via `SKIP LOCKED`.
*   **Tenant Quotas:** Admins can set maximum lease counts per team.
*   **Zombie Reclamation:** Automatic detection of crashed clients via missing heartbeats.
*   **Auto-Draining:** Background health checks automatically take broken hardware offline.
*   **Outbox Notifications:** Reliable events for integration with Slack, PagerDuty, or CI/CD.

