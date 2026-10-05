#ifndef SIMPLE_SERVER_ENGINE_H
#define SIMPLE_SERVER_ENGINE_H
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

enum { SS_PENDING = 0, SS_READY = 1, SS_PANICKED = 2, SS_CANCELLED = 3, SS_INVALID = 4 };

/* Each owned value is released by its producer, exactly once. No callbacks may
 * unwind. Handles must originate from the matching constructor. */
typedef struct {
    const uint8_t *data;
    size_t len;
    void *context;
    void (*release)(void *);
} ss_buffer;

/* wake borrows the reference; release consumes it. Both are thread safe. */
typedef struct {
    void *context;
    void (*wake)(void *);
    void (*release)(void *);
} ss_wake;

/* poll consumes its wake, including when it returns an error. */
typedef struct {
    void *context;
    int32_t (*poll)(void *, ss_wake);
    void (*release)(void *);
} ss_task;

typedef struct {
    size_t size;
    uint32_t worker_threads;
    uint8_t start_paused;
} ss_runtime_options;

typedef struct {
    void *context;
    int32_t (*poll)(void *, ss_wake, ss_buffer *);
    void (*release)(void *);
} ss_bytes_future;

typedef struct {
    void *context;
    ss_bytes_future (*call)(void *, const uint8_t *, size_t);
    void (*release)(void *);
} ss_callback;

/* Process-local monotonic time: signed seconds plus nanos in [0, 1000000000). */
typedef struct {
    int64_t seconds;
    uint32_t nanoseconds;
} ss_monotonic_instant;

typedef struct {
    size_t size;
    uint32_t abi_version;
    ss_buffer (*version)(void);
    ss_buffer (*last_error)(void);
    void *(*runtime_new)(ss_runtime_options);
    void (*runtime_release)(void *);
    /* Borrows the root task; it remains on the calling thread. */
    int32_t (*runtime_run)(void *, ss_task);
    /* Consumes a movable task, even on error. */
    void *(*runtime_spawn)(void *, ss_task);
    void (*task_abort)(void *);
    void (*task_release)(void *);
    void *(*operation_new)(void *, const uint8_t *, size_t);
    /* Output is initialized only for SS_READY; poll consumes wake. */
    int32_t (*operation_poll)(void *, ss_wake, ss_buffer *);
    void (*operation_release)(void *);
    int32_t (*resource_new)(const uint8_t *, size_t, ss_buffer *);
    void (*resource_release)(uint32_t, uint64_t);
    void *(*runtime_spawn_blocking)(void *, ss_task);
    uint64_t (*callback_new)(ss_callback);
    /* Null runtime reads real time; output initialized only for SS_READY. */
    int32_t (*clock_now)(void *, ss_monotonic_instant *);
    uint8_t (*clock_valid)(ss_monotonic_instant);
} ss_api_v1;

/* Unpublished development ABI; freeze table and protocol together at release. */
const ss_api_v1 *simple_server_engine_v1(void);

#ifdef __cplusplus
}
#endif
#endif
