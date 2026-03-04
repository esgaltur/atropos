import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
    vus: 100, // 100 virtual users
    duration: '30s', // sustain for 30 seconds
};

const BASE_URL = 'http://localhost:3000';

export default function () {
    const payload = JSON.stringify({
        pool_type: "A100",
        owner_id: `user-${__VU}`,
        tenant_id: "load-test-dept",
        ttl_seconds: 60
    });

    const params = {
        headers: {
            'Content-Type': 'application/json',
        },
    };

    // Attempt allocation
    const res = http.post(`${BASE_URL}/leases`, payload, params);

    // We expect either 200 (Success) or 409 (Waitlisted/Conflict)
    check(res, {
        'is status 200 or 409': (r) => r.status === 200 || r.status === 409,
    });

    sleep(1);
}
