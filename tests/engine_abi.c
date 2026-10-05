#include "simple_server_engine.h"
#include <assert.h>
#include <string.h>

static int32_t poll_root(void *context, ss_wake wake) {
    unsigned *count = context;
    ++*count;
    wake.release(wake.context);
    return SS_READY;
}
static void release_root(void *context) {
    (void)context;
    assert(0 && "runtime_run must borrow the root");
}
int main(void) {
    const ss_api_v1 *api = simple_server_engine_v1();
    assert(api && api->size == sizeof(ss_api_v1) && api->abi_version == 1);
    ss_buffer version = api->version();
    assert(version.len == 5 && memcmp(version.data, "0.2.0", 5) == 0);
    version.release(version.context);
    ss_runtime_options options = { sizeof(ss_runtime_options), 0, 0 };
    void *runtime = api->runtime_new(options);
    assert(runtime);
    ss_monotonic_instant now;
    assert(api->clock_now(runtime, &now) == SS_READY);
    assert(api->clock_valid(now) == 1);
    now.nanoseconds = 1000000000;
    assert(api->clock_valid(now) == 0);
    unsigned calls = 0;
    ss_task root = { &calls, poll_root, release_root };
    assert(api->runtime_run(runtime, root) == SS_READY && calls == 1);
    api->runtime_release(runtime);
    return 0;
}
