# API Integration Guide

This guide provides practical `curl` examples for interacting with the Atropos Resource Allocator platform.

## 1. System Health
Verify the API and database are online.
```bash
curl -X GET http://localhost:3000/health
```

## 2. Infrastructure Setup (Admin)

### Create a Pool
Pools define a group of identical resources and their allocation policy.
```bash
curl -X POST http://localhost:3000/pools 
  -H "Content-Type: application/json" 
  -d '{
    "name": "NVIDIA-A100-Cluster",
    "resource_type": "GPU-A100",
    "policy": "FIFO"
  }'
```
*Note the returned `id` (e.g., `123e4567-e89b-12d3-a456-426614174000`), you need it for the next step.*

### Register a Resource
Attach a physical or logical resource to the pool you just created.
```bash
curl -X POST http://localhost:3000/resources 
  -H "Content-Type: application/json" 
  -d '{
    "pool_id": "123e4567-e89b-12d3-a456-426614174000",
    "external_id": "rack-01-node-A",
    "attributes": {
        "vram": "80GB",
        "pcie": "gen4"
    }
  }'
```

## 3. Core Workflow (Tenants)

### Allocate a Lease
Request an exclusive lock on a resource for 3600 seconds (1 hour).
```bash
curl -X POST http://localhost:3000/leases 
  -H "Content-Type: application/json" 
  -d '{
    "pool_type": "GPU-A100",
    "owner_id": "data-science-team",
    "tenant_id": "project-apollo",
    "ttl_seconds": 3600,
    "waitlist": true,
    "preempt": false
  }'
```
*If successful (HTTP 200), the response contains the `lease_id` and `resource_id`. If full, it returns HTTP 409.*

### Renew a Lease
Extend an active lease by 1800 seconds (30 minutes).
```bash
curl -X POST http://localhost:3000/leases/<LEASE_ID>/renew 
  -H "Content-Type: application/json" 
  -d '{
    "extension_seconds": 1800
  }'
```

### Release a Lease
Return the resource to the pool before the TTL expires.
```bash
curl -X DELETE http://localhost:3000/leases/<LEASE_ID>
```
