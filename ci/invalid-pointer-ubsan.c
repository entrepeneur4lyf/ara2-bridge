#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#if defined(__unix__) || defined(__APPLE__)
#include <sys/mman.h>
#include <unistd.h>
#endif

static int null_adjacent(void) {
    volatile size_t value = *(const size_t *)(uintptr_t)1;
    return (int)value;
}

#if defined(__unix__) || defined(__APPLE__)
static int unreadable(void) {
    const size_t page_size = (size_t)sysconf(_SC_PAGESIZE);
    void *page = mmap(NULL, page_size, PROT_NONE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (page == MAP_FAILED) {
        return 65;
    }
    volatile size_t value = *(const size_t *)page;
    return (int)value;
}

static int guard_page(void) {
    const size_t page_size = (size_t)sysconf(_SC_PAGESIZE);
    unsigned char *mapping = mmap(
        NULL,
        page_size * 2,
        PROT_READ | PROT_WRITE,
        MAP_PRIVATE | MAP_ANONYMOUS,
        -1,
        0);
    if (mapping == MAP_FAILED || mprotect(mapping + page_size, page_size, PROT_NONE) != 0) {
        return 65;
    }
    *(size_t *)(mapping + page_size - sizeof(size_t)) = page_size;
    volatile unsigned char value = mapping[page_size];
    return (int)value;
}
#else
static int unreadable(void) { return null_adjacent(); }
static int guard_page(void) { return null_adjacent(); }
#endif

int main(int argc, char **argv) {
    if (argc != 2) {
        return 64;
    }
    if (strcmp(argv[1], "null-adjacent") == 0) {
        return null_adjacent();
    }
    if (strcmp(argv[1], "unreadable") == 0) {
        return unreadable();
    }
    if (strcmp(argv[1], "guard-page") == 0) {
        return guard_page();
    }
    return 64;
}
