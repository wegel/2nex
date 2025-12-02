/*
 * nex-ld-shim-raw.c - Minimal dynamic linker shim for 2nex
 *
 * This version uses NO libc - only raw syscalls.
 * Compile with: gcc -nostdlib -nostartfiles -static -O2 -o nex-ld-shim nex-ld-shim-raw.c
 *
 * When used as PT_INTERP, the kernel sets up the stack as:
 *   [rsp+0]  = argc
 *   [rsp+8]  = argv[0], argv[1], ..., NULL
 *   then envp[0], envp[1], ..., NULL
 *   then auxv pairs (type, value), ending with AT_NULL
 *
 * This shim loads the real ld-linux into memory and jumps to it, rather than
 * using execve. This preserves /proc/self/exe pointing to the original binary,
 * which is required for programs that re-exec themselves (like nvim's TUI).
 */

// auxv types we care about
#define AT_NULL     0
#define AT_PHDR     3
#define AT_PHENT    4
#define AT_PHNUM    5
#define AT_BASE     7
#define AT_ENTRY    9
#define AT_SECURE   23
#define AT_EXECFN   31

// syscall numbers (x86_64)
#define SYS_read      0
#define SYS_write     1
#define SYS_open      2
#define SYS_close     3
#define SYS_stat      4
#define SYS_fstat     5
#define SYS_lstat     6
#define SYS_mmap      9
#define SYS_mprotect  10
#define SYS_munmap    11
#define SYS_readlink  89
#define SYS_execve    59
#define SYS_exit      60

// open flags
#define O_RDONLY 0

// mmap flags
#define PROT_NONE   0x0
#define PROT_READ   0x1
#define PROT_WRITE  0x2
#define PROT_EXEC   0x4
#define MAP_PRIVATE   0x02
#define MAP_FIXED     0x10
#define MAP_ANONYMOUS 0x20

// ELF types
#define PT_NULL    0
#define PT_LOAD    1
#define PT_DYNAMIC 2
#define PT_INTERP  3
#define PT_PHDR    6

#define PF_X 0x1
#define PF_W 0x2
#define PF_R 0x4

#define ET_DYN 3

typedef struct {
    unsigned char e_ident[16];
    unsigned short e_type;
    unsigned short e_machine;
    unsigned int e_version;
    unsigned long e_entry;
    unsigned long e_phoff;
    unsigned long e_shoff;
    unsigned int e_flags;
    unsigned short e_ehsize;
    unsigned short e_phentsize;
    unsigned short e_phnum;
    unsigned short e_shentsize;
    unsigned short e_shnum;
    unsigned short e_shstrndx;
} Elf64_Ehdr;

typedef struct {
    unsigned int p_type;
    unsigned int p_flags;
    unsigned long p_offset;
    unsigned long p_vaddr;
    unsigned long p_paddr;
    unsigned long p_filesz;
    unsigned long p_memsz;
    unsigned long p_align;
} Elf64_Phdr;

// stat structure (simplified, only need st_mode)
struct stat {
    unsigned long st_dev;
    unsigned long st_ino;
    unsigned long st_nlink;
    unsigned int  st_mode;
    unsigned int  st_uid;
    unsigned int  st_gid;
    unsigned int  __pad0;
    unsigned long st_rdev;
    long          st_size;
    long          st_blksize;
    long          st_blocks;
    // ... rest not needed
    char __pad[64];
};

#define S_IFMT   0170000
#define S_IFREG  0100000
#define S_ISREG(m) (((m) & S_IFMT) == S_IFREG)

// raw syscall wrappers
static inline long syscall1(long n, long a1) {
    long ret;
    __asm__ volatile (
        "syscall"
        : "=a"(ret)
        : "a"(n), "D"(a1)
        : "rcx", "r11", "memory"
    );
    return ret;
}

static inline long syscall2(long n, long a1, long a2) {
    long ret;
    __asm__ volatile (
        "syscall"
        : "=a"(ret)
        : "a"(n), "D"(a1), "S"(a2)
        : "rcx", "r11", "memory"
    );
    return ret;
}

static inline long syscall3(long n, long a1, long a2, long a3) {
    long ret;
    register long r10 __asm__("r10") = a3;
    __asm__ volatile (
        "syscall"
        : "=a"(ret)
        : "a"(n), "D"(a1), "S"(a2), "d"(a3)
        : "rcx", "r11", "memory"
    );
    (void)r10;
    return ret;
}

static inline long syscall3_r10(long n, long a1, long a2, long a3) {
    long ret;
    register long r10 __asm__("r10") = a3;
    __asm__ volatile (
        "syscall"
        : "=a"(ret)
        : "a"(n), "D"(a1), "S"(a2), "r"(r10)
        : "rcx", "r11", "memory"
    );
    return ret;
}

static inline long syscall6(long n, long a1, long a2, long a3, long a4, long a5, long a6) {
    long ret;
    register long r10 __asm__("r10") = a4;
    register long r8  __asm__("r8")  = a5;
    register long r9  __asm__("r9")  = a6;
    __asm__ volatile (
        "syscall"
        : "=a"(ret)
        : "a"(n), "D"(a1), "S"(a2), "d"(a3), "r"(r10), "r"(r8), "r"(r9)
        : "rcx", "r11", "memory"
    );
    return ret;
}

// syscall wrappers
static inline void sys_exit(int code) {
    syscall1(SYS_exit, code);
    __builtin_unreachable();
}

static inline long sys_write(int fd, const char *buf, unsigned long count) {
    return syscall3(SYS_write, fd, (long)buf, count);
}

static inline long sys_read(int fd, char *buf, unsigned long count) {
    return syscall3(SYS_read, fd, (long)buf, count);
}

static inline long sys_open(const char *path, int flags) {
    return syscall2(SYS_open, (long)path, flags);
}

static inline long sys_close(int fd) {
    return syscall1(SYS_close, fd);
}

static inline long sys_stat(const char *path, struct stat *st) {
    return syscall2(SYS_stat, (long)path, (long)st);
}

static inline long sys_readlink(const char *path, char *buf, unsigned long bufsiz) {
    return syscall3(SYS_readlink, (long)path, (long)buf, bufsiz);
}

static inline long sys_execve(const char *path, char **argv, char **envp) {
    return syscall3(SYS_execve, (long)path, (long)argv, (long)envp);
}

static inline void *sys_mmap(void *addr, unsigned long length, int prot,
                             int flags, int fd, long offset) {
    return (void *)syscall6(SYS_mmap, (long)addr, length, prot, flags, fd, offset);
}

static inline long sys_mprotect(void *addr, unsigned long length, int prot) {
    return syscall3(SYS_mprotect, (long)addr, length, prot);
}

static inline long sys_munmap(void *addr, unsigned long length) {
    return syscall2(SYS_munmap, (long)addr, length);
}

// string functions
static unsigned long str_len(const char *s) {
    unsigned long n = 0;
    while (s[n]) n++;
    return n;
}

static int str_eq(const char *a, const char *b) {
    while (*a && *b && *a == *b) { a++; b++; }
    return *a == *b;
}

static int str_has_prefix(const char *s, const char *prefix) {
    while (*prefix) {
        if (*s != *prefix) return 0;
        s++; prefix++;
    }
    return 1;
}

static void str_copy(char *dst, const char *src, unsigned long max) {
    unsigned long i = 0;
    while (i < max - 1 && src[i]) {
        dst[i] = src[i];
        i++;
    }
    dst[i] = '\0';
}

static void str_append(char *dst, const char *src, unsigned long max) {
    unsigned long dlen = str_len(dst);
    unsigned long i = 0;
    while (dlen + i < max - 1 && src[i]) {
        dst[dlen + i] = src[i];
        i++;
    }
    dst[dlen + i] = '\0';
}

// error output
static void write_str(int fd, const char *s) {
    sys_write(fd, s, str_len(s));
}

static void fatal(const char *msg) {
    write_str(2, "nex-ld-shim: ");
    write_str(2, msg);
    write_str(2, "\n");
    sys_exit(1);
}

static void fatal_path(const char *msg, const char *path) {
    write_str(2, "nex-ld-shim: ");
    write_str(2, msg);
    write_str(2, ": ");
    write_str(2, path);
    write_str(2, "\n");
    sys_exit(1);
}

// check if file exists and is regular
static int file_exists(const char *path) {
    struct stat st;
    if (sys_stat(path, &st) == 0 && S_ISREG(st.st_mode)) {
        return 1;
    }
    return 0;
}

// check if file is an ELF binary (magic bytes: 0x7f 'E' 'L' 'F')
static int is_elf(const char *path) {
    int fd = sys_open(path, O_RDONLY);
    if (fd < 0) return 0;

    char magic[4];
    long n = sys_read(fd, magic, 4);
    sys_close(fd);

    if (n != 4) return 0;
    return (magic[0] == 0x7f && magic[1] == 'E' && magic[2] == 'L' && magic[3] == 'F');
}

// find last slash in path, return pointer to char after it (or start if no slash)
static char *path_basename(char *path) {
    char *last = path;
    for (char *p = path; *p; p++) {
        if (*p == '/') last = p + 1;
    }
    return last;
}

// buffer sizes - keep small to avoid stack overflow
#define PATH_MAX 1024
#define ARG_MAX 8192

// truncate path at last slash (dirname in-place)
static void path_dirname(char *path) {
    char *last_slash = (void*)0;
    for (char *p = path; *p; p++) {
        if (*p == '/') last_slash = p;
    }
    if (last_slash) {
        if (last_slash == path) {
            // root directory
            path[1] = '\0';
        } else {
            *last_slash = '\0';
        }
    }
}

// normalize path in-place: resolve . and .. components
static void path_normalize(char *path) {
    if (!path || !path[0]) return;

    // work with components
    char *out = path;
    char *in = path;

    // preserve leading slash
    if (*in == '/') {
        *out++ = *in++;
    }

    while (*in) {
        // skip leading slashes
        while (*in == '/') in++;
        if (!*in) break;

        // find component end
        char *comp_start = in;
        while (*in && *in != '/') in++;
        unsigned long comp_len = in - comp_start;

        if (comp_len == 1 && comp_start[0] == '.') {
            // skip "."
            continue;
        }

        if (comp_len == 2 && comp_start[0] == '.' && comp_start[1] == '.') {
            // go back one directory
            if (out > path + 1) {
                out--;  // back over trailing slash
                while (out > path && *(out - 1) != '/') out--;
            }
            continue;
        }

        // add separator if needed
        if (out > path && *(out - 1) != '/') {
            *out++ = '/';
        }

        // copy component
        for (unsigned long i = 0; i < comp_len; i++) {
            *out++ = comp_start[i];
        }
    }

    // ensure we have at least "/"
    if (out == path) {
        *out++ = '/';
    }
    *out = '\0';
}

// resolve a single symlink (if it is one), returns new length or -1 on too many links
// modifies path in place
static int resolve_one_symlink(char *path, int *links_left) {
    char link_target[PATH_MAX];
    char temp[PATH_MAX];

    long len = sys_readlink(path, link_target, sizeof(link_target) - 1);
    if (len < 0) {
        // not a symlink
        return str_len(path);
    }

    if (--(*links_left) <= 0) {
        return -1;  // too many symlinks
    }

    link_target[len] = '\0';

    if (link_target[0] == '/') {
        // absolute symlink
        str_copy(path, link_target, PATH_MAX);
    } else {
        // relative symlink: prepend directory of current path
        str_copy(temp, path, sizeof(temp));
        path_dirname(temp);

        if (str_eq(temp, "/")) {
            str_copy(path, "/", PATH_MAX);
            str_append(path, link_target, PATH_MAX);
        } else {
            str_copy(path, temp, PATH_MAX);
            str_append(path, "/", PATH_MAX);
            str_append(path, link_target, PATH_MAX);
        }
    }

    path_normalize(path);
    return str_len(path);
}

// realpath implementation: resolve ALL symlinks in path (intermediate and final)
// this properly handles symlinks in directory components, not just the final file
// uses recursion to handle symlinks that themselves contain paths with symlinks
static int resolve_realpath_recursive(const char *start_path, char *result, unsigned long result_size, int *links_left) {
    char resolved[PATH_MAX];
    char component[PATH_MAX];
    char link_target[PATH_MAX];
    char new_path[PATH_MAX];

    // start with "/" for absolute paths
    if (start_path[0] == '/') {
        str_copy(resolved, "/", sizeof(resolved));
    } else {
        resolved[0] = '\0';
    }

    const char *p = start_path;
    if (*p == '/') p++;  // skip leading slash

    while (*p) {
        // extract next component
        const char *comp_start = p;
        while (*p && *p != '/') p++;
        unsigned long comp_len = p - comp_start;

        if (comp_len == 0) {
            if (*p == '/') p++;
            continue;
        }

        // copy component
        if (comp_len >= sizeof(component)) comp_len = sizeof(component) - 1;
        for (unsigned long i = 0; i < comp_len; i++) {
            component[i] = comp_start[i];
        }
        component[comp_len] = '\0';

        // handle . and ..
        if (str_eq(component, ".")) {
            if (*p == '/') p++;
            continue;
        }
        if (str_eq(component, "..")) {
            path_dirname(resolved);
            if (resolved[0] == '\0') {
                str_copy(resolved, "/", sizeof(resolved));
            }
            if (*p == '/') p++;
            continue;
        }

        // append component to resolved path
        if (!str_eq(resolved, "/") && resolved[0] != '\0') {
            str_append(resolved, "/", sizeof(resolved));
        }
        str_append(resolved, component, sizeof(resolved));

        // check if this path is a symlink
        long len = sys_readlink(resolved, link_target, sizeof(link_target) - 1);
        if (len >= 0) {
            // it's a symlink - need to resolve the target plus remaining path
            if (--(*links_left) <= 0) {
                return -1;  // too many symlinks
            }
            link_target[len] = '\0';

            // build new path: resolved symlink target + remaining components
            if (link_target[0] == '/') {
                // absolute symlink
                str_copy(new_path, link_target, sizeof(new_path));
            } else {
                // relative symlink: resolve relative to parent dir
                char parent[PATH_MAX];
                str_copy(parent, resolved, sizeof(parent));
                path_dirname(parent);
                if (str_eq(parent, "") || parent[0] == '\0') {
                    str_copy(new_path, "/", sizeof(new_path));
                } else {
                    str_copy(new_path, parent, sizeof(new_path));
                }
                str_append(new_path, "/", sizeof(new_path));
                str_append(new_path, link_target, sizeof(new_path));
            }

            // append remaining path components
            if (*p == '/') p++;
            if (*p) {
                str_append(new_path, "/", sizeof(new_path));
                str_append(new_path, p, sizeof(new_path));
            }

            // normalize to handle .. in symlink target
            path_normalize(new_path);

            // recursively resolve the new path
            return resolve_realpath_recursive(new_path, result, result_size, links_left);
        }

        if (*p == '/') p++;
    }

    if (resolved[0] == '\0') {
        str_copy(resolved, "/", sizeof(resolved));
    }

    str_copy(result, resolved, result_size);
    return str_len(result);
}

static int resolve_realpath(const char *start_path, char *result, unsigned long result_size) {
    int links_left = 40;
    return resolve_realpath_recursive(start_path, result, result_size, &links_left);
}

// resolve symlink chain without /proc, returns length or -1 on error
// handles both absolute and relative symlinks
static int resolve_symlink_chain(const char *start_path, char *result, unsigned long result_size) {
    return resolve_realpath(start_path, result, result_size);
}

// round down to page boundary
static inline unsigned long page_align_down(unsigned long addr) {
    return addr & ~0xFFFUL;
}

// round up to page boundary
static inline unsigned long page_align_up(unsigned long addr) {
    return (addr + 0xFFF) & ~0xFFFUL;
}

// convert ELF pflags to mmap prot flags
static inline int elf_prot(unsigned int pflags) {
    int prot = 0;
    if (pflags & PF_R) prot |= PROT_READ;
    if (pflags & PF_W) prot |= PROT_WRITE;
    if (pflags & PF_X) prot |= PROT_EXEC;
    return prot;
}

// load ld-linux into memory and return its entry point
// returns 0 on failure
static unsigned long load_elf_interp(const char *path, unsigned long *out_base) {
    Elf64_Ehdr ehdr;
    Elf64_Phdr phdrs[16];  // should be enough for ld-linux

    int fd = sys_open(path, O_RDONLY);
    if (fd < 0) return 0;

    // read ELF header
    if (sys_read(fd, (char *)&ehdr, sizeof(ehdr)) != sizeof(ehdr)) {
        sys_close(fd);
        return 0;
    }

    // verify ELF magic
    if (ehdr.e_ident[0] != 0x7f || ehdr.e_ident[1] != 'E' ||
        ehdr.e_ident[2] != 'L' || ehdr.e_ident[3] != 'F') {
        sys_close(fd);
        return 0;
    }

    // must be ET_DYN (shared object / PIE)
    if (ehdr.e_type != ET_DYN) {
        sys_close(fd);
        return 0;
    }

    // read program headers
    if (ehdr.e_phnum > 16) {
        sys_close(fd);
        return 0;
    }

    // seek to phdr offset - we need to use pread or re-open, but since we don't
    // have lseek, we'll close and re-read. simpler: read entire file is bad.
    // actually, we can just read from start and skip.
    sys_close(fd);
    fd = sys_open(path, O_RDONLY);
    if (fd < 0) return 0;

    // skip to phoff
    char skip_buf[512];
    unsigned long to_skip = ehdr.e_phoff;
    while (to_skip > 0) {
        unsigned long chunk = to_skip > sizeof(skip_buf) ? sizeof(skip_buf) : to_skip;
        if (sys_read(fd, skip_buf, chunk) != (long)chunk) {
            sys_close(fd);
            return 0;
        }
        to_skip -= chunk;
    }

    unsigned long phdr_size = ehdr.e_phnum * sizeof(Elf64_Phdr);
    if (sys_read(fd, (char *)phdrs, phdr_size) != (long)phdr_size) {
        sys_close(fd);
        return 0;
    }
    sys_close(fd);

    // find the extent of all PT_LOAD segments
    unsigned long vaddr_min = (unsigned long)-1;
    unsigned long vaddr_max = 0;

    for (int i = 0; i < ehdr.e_phnum; i++) {
        if (phdrs[i].p_type != PT_LOAD) continue;
        if (phdrs[i].p_vaddr < vaddr_min) vaddr_min = phdrs[i].p_vaddr;
        unsigned long end = phdrs[i].p_vaddr + phdrs[i].p_memsz;
        if (end > vaddr_max) vaddr_max = end;
    }

    if (vaddr_min >= vaddr_max) return 0;

    // reserve address space for the entire range (we'll map over it)
    unsigned long map_size = page_align_up(vaddr_max) - page_align_down(vaddr_min);
    void *base = sys_mmap((void *)0, map_size, PROT_NONE,
                          MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if ((long)base < 0 && (long)base > -4096) {
        return 0;
    }

    unsigned long load_bias = (unsigned long)base - page_align_down(vaddr_min);

    // now map each PT_LOAD segment
    fd = sys_open(path, O_RDONLY);
    if (fd < 0) {
        sys_munmap(base, map_size);
        return 0;
    }

    for (int i = 0; i < ehdr.e_phnum; i++) {
        if (phdrs[i].p_type != PT_LOAD) continue;

        unsigned long vaddr = phdrs[i].p_vaddr;
        unsigned long offset = phdrs[i].p_offset;
        unsigned long filesz = phdrs[i].p_filesz;
        unsigned long memsz = phdrs[i].p_memsz;

        // ELF requires: (vaddr - offset) % page_size == 0
        // but vaddr and offset may differ by a multiple of page_size
        // (this happens with separate-code in modern glibc)

        // page-align for mmap
        unsigned long map_start = page_align_down(vaddr);

        // offset within the page
        unsigned long page_offset = vaddr & 0xFFF;

        // size to map includes the page offset
        unsigned long map_size_seg = page_align_up(memsz + page_offset);

        // first, map anonymous memory for the whole segment
        void *mapped = sys_mmap((void *)(load_bias + map_start),
                               map_size_seg,
                               PROT_READ | PROT_WRITE,
                               MAP_PRIVATE | MAP_FIXED | MAP_ANONYMOUS,
                               -1, 0);
        if ((long)mapped < 0 && (long)mapped > -4096) {
            sys_close(fd);
            sys_munmap(base, map_size);
            return 0;
        }

        // now read the file contents into the mapped area
        // we need to read filesz bytes at file offset 'offset' to vaddr
        if (filesz > 0) {
            // seek to offset by reading and discarding
            // (we already have fd open, need to seek)
            sys_close(fd);
            fd = sys_open(path, O_RDONLY);
            if (fd < 0) {
                sys_munmap(base, map_size);
                return 0;
            }

            // skip to the right offset
            char skip_buf[512];
            unsigned long to_skip = offset;
            while (to_skip > 0) {
                unsigned long chunk = to_skip > sizeof(skip_buf) ? sizeof(skip_buf) : to_skip;
                if (sys_read(fd, skip_buf, chunk) != (long)chunk) {
                    sys_close(fd);
                    sys_munmap(base, map_size);
                    return 0;
                }
                to_skip -= chunk;
            }

            // read the file contents
            char *dest = (char *)(load_bias + vaddr);
            unsigned long remaining = filesz;
            while (remaining > 0) {
                long n = sys_read(fd, dest, remaining);
                if (n <= 0) {
                    sys_close(fd);
                    sys_munmap(base, map_size);
                    return 0;
                }
                dest += n;
                remaining -= n;
            }
        }

        // bss is already zeroed by MAP_ANONYMOUS

        // set final protections
        sys_mprotect((void *)(load_bias + map_start), map_size_seg, elf_prot(phdrs[i].p_flags));
    }

    sys_close(fd);

    *out_base = load_bias;
    return load_bias + ehdr.e_entry;
}

// main logic
static void shim_main(int argc, char **argv, char **envp, unsigned long *auxv,
                      unsigned long *stack_bottom) {
    char resolved_exe[PATH_MAX];
    char app_root[PATH_MAX];
    char loader_path[PATH_MAX];
    char lib_dir[PATH_MAX];
    char sentinel[PATH_MAX];
    char dir[PATH_MAX];

    // find AT_EXECFN from auxv
    const char *execfn = (void*)0;

    for (unsigned long *a = auxv; a[0] != AT_NULL; a += 2) {
        if (a[0] == AT_EXECFN) execfn = (const char *)a[1];
    }

    // note: setuid/setgid (AT_SECURE) is safe because we don't read any
    // environment variables for library paths - paths are computed solely
    // from the binary location which the kernel provides via AT_EXECFN

    if (!execfn) {
        fatal("AT_EXECFN not available");
    }

    // resolve executable path by following the symlink chain
    // this handles relative symlinks without needing /proc
    int len = resolve_symlink_chain(execfn, resolved_exe, sizeof(resolved_exe));
    if (len < 0) {
        fatal_path("too many symlinks", execfn);
    }

    // if resolved path is not an ELF (e.g., a shell script), use argv[0] instead
    // this handles shebang scripts where AT_EXECFN points to the script but argv[0] is the shell
    if (!is_elf(resolved_exe)) {
        // argv[0] should be the actual interpreter (e.g., /bin/sh)
        // resolve symlink chain for argv[0]
        len = resolve_symlink_chain(argv[0], resolved_exe, sizeof(resolved_exe));
        if (len < 0) {
            fatal_path("too many symlinks for interpreter", argv[0]);
        }
        // verify it's now an ELF
        if (!is_elf(resolved_exe)) {
            fatal_path("cannot resolve interpreter", argv[0]);
        }
    }

    // find .nex-app-root by walking up
    str_copy(dir, resolved_exe, sizeof(dir));
    path_dirname(dir);  // start with parent directory of executable

    int found = 0;
    while (1) {
        str_copy(sentinel, dir, sizeof(sentinel));
        str_append(sentinel, "/.nex-app-root", sizeof(sentinel));

        if (file_exists(sentinel)) {
            str_copy(app_root, dir, sizeof(app_root));
            found = 1;
            break;
        }

        // at root?
        if (str_eq(dir, "/")) {
            break;
        }

        path_dirname(dir);
    }

    if (!found) {
        fatal_path(".nex-app-root not found for", resolved_exe);
    }

    // build lib_dir with multiple search paths (colon-separated)
    // order: app_root/lib : app_root/usr/lib : app_root/usr/lib/systemd
    str_copy(lib_dir, app_root, sizeof(lib_dir));
    str_append(lib_dir, "/lib:", sizeof(lib_dir));
    str_append(lib_dir, app_root, sizeof(lib_dir));
    str_append(lib_dir, "/usr/lib:", sizeof(lib_dir));
    str_append(lib_dir, app_root, sizeof(lib_dir));
    str_append(lib_dir, "/usr/lib/systemd", sizeof(lib_dir));

    // build loader_path from primary lib dir
    str_copy(loader_path, app_root, sizeof(loader_path));
    str_append(loader_path, "/lib/ld-linux-x86-64.so.2", sizeof(loader_path));

    // check for nex.loader override
    char nex_loader_file[PATH_MAX];
    str_copy(nex_loader_file, app_root, sizeof(nex_loader_file));
    str_append(nex_loader_file, "/nex.loader", sizeof(nex_loader_file));

    if (file_exists(nex_loader_file)) {
        int fd = sys_open(nex_loader_file, O_RDONLY);
        if (fd >= 0) {
            char buf[PATH_MAX];
            long n = sys_read(fd, buf, sizeof(buf) - 1);
            sys_close(fd);
            if (n > 0) {
                buf[n] = '\0';
                // trim newline
                for (int i = 0; buf[i]; i++) {
                    if (buf[i] == '\n' || buf[i] == '\r') {
                        buf[i] = '\0';
                        break;
                    }
                }
                str_copy(loader_path, buf, sizeof(loader_path));
            }
        }
    }

    if (!file_exists(loader_path)) {
        fatal_path("loader not found", loader_path);
    }

    // load ld-linux into memory
    unsigned long interp_base = 0;
    unsigned long entry = load_elf_interp(loader_path, &interp_base);
    if (!entry) {
        fatal_path("failed to load interpreter", loader_path);
    }

    // update auxv: set AT_BASE to where we loaded the interpreter
    // the kernel already set AT_PHDR, AT_ENTRY etc. for the main program
    for (unsigned long *a = auxv; a[0] != AT_NULL; a += 2) {
        if (a[0] == AT_BASE) {
            a[1] = interp_base;
        }
    }

    // filter envp and add LD_LIBRARY_PATH
    // since we're jumping to ld-linux as PT_INTERP (not command-line),
    // it will use LD_LIBRARY_PATH to find libraries
    static char *new_envp[4096];
    int env_idx = 0;

    for (char **e = envp; *e && env_idx < 4090; e++) {
        if (str_has_prefix(*e, "LD_LIBRARY_PATH=") ||
            str_has_prefix(*e, "LD_PRELOAD=") ||
            str_has_prefix(*e, "LD_AUDIT=")) {
            continue;
        }
        new_envp[env_idx++] = *e;
    }

    // add our controlled LD_LIBRARY_PATH
    static char ld_library_path_env[PATH_MAX + 20];
    str_copy(ld_library_path_env, "LD_LIBRARY_PATH=", sizeof(ld_library_path_env));
    str_append(ld_library_path_env, lib_dir, sizeof(ld_library_path_env));
    new_envp[env_idx++] = ld_library_path_env;

    // add NEX_APP_ROOT and NEX_LIB_DIR
    static char app_root_env[PATH_MAX + 16];
    static char lib_dir_env[PATH_MAX + 16];

    str_copy(app_root_env, "NEX_APP_ROOT=", sizeof(app_root_env));
    str_append(app_root_env, app_root, sizeof(app_root_env));
    new_envp[env_idx++] = app_root_env;

    str_copy(lib_dir_env, "NEX_LIB_DIR=", sizeof(lib_dir_env));
    str_append(lib_dir_env, lib_dir, sizeof(lib_dir_env));
    new_envp[env_idx++] = lib_dir_env;

    new_envp[env_idx] = (void*)0;

    // rebuild the stack with new envp
    // stack layout: argc, argv[0..argc-1], NULL, envp[0..n], NULL, auxv
    // we need to replace envp in place or rebuild the stack

    // IMPORTANT: we need to build the new stack in a separate area first,
    // because the original argv/envp pointers point to string data that
    // may be on the stack above our write area

    // count new_envp entries
    int new_env_count = env_idx;

    // find the end of auxv
    unsigned long *auxv_end = auxv;
    while (auxv_end[0] != AT_NULL) auxv_end += 2;
    auxv_end += 2;  // include the AT_NULL entry

    // use static buffer to build new stack frame
    // this avoids overwriting data we still need to read
    static unsigned long new_stack[8192];
    unsigned long *new_sp = new_stack;

    // write argc
    *new_sp++ = argc;

    // write argv pointers (these point to strings in the original stack's
    // string area, which is above the pointers and won't be overwritten)
    for (int i = 0; i < argc; i++) {
        *new_sp++ = (unsigned long)argv[i];
    }
    *new_sp++ = 0;  // NULL terminator

    // write new envp pointers
    for (int i = 0; i < new_env_count; i++) {
        *new_sp++ = (unsigned long)new_envp[i];
    }
    *new_sp++ = 0;  // NULL terminator

    // copy auxv (already modified AT_BASE in place)
    for (unsigned long *a = auxv; a < auxv_end; a++) {
        *new_sp++ = *a;
    }

    // now copy the built stack to the original stack location
    // we go backwards (from end to start) to avoid reading from
    // already-written memory if there's overlap
    unsigned long *dst = stack_bottom;
    for (unsigned long i = 0; i < (unsigned long)(new_sp - new_stack); i++) {
        dst[i] = new_stack[i];
    }

    // jump to ld-linux entry point with new stack
    // ld-linux expects rsp to point to argc on the stack
    __asm__ volatile (
        "mov %0, %%rsp\n"
        "xor %%rdx, %%rdx\n"   // clear rdx (rtld_fini)
        "jmp *%1\n"
        :
        : "r"(stack_bottom), "r"(entry)
        : "memory"
    );

    __builtin_unreachable();
}

// wrapper called from asm _start
void _start_c(unsigned long *sp) {
    int argc = (int)sp[0];
    char **argv = (char **)(sp + 1);
    char **envp = argv + argc + 1;

    // find auxv (skip envp)
    char **e = envp;
    while (*e) e++;
    unsigned long *auxv = (unsigned long *)(e + 1);

    // pass sp as stack_bottom - shim_main will rebuild the stack there
    shim_main(argc, argv, envp, auxv, sp);

    // should never reach here - shim_main jumps to ld-linux
    sys_exit(1);
}

// entry point in assembly - gets stack pointer and calls C
__asm__(
    ".global _start\n"
    "_start:\n"
    "    xor %rbp, %rbp\n"        // clear frame pointer (ABI)
    "    mov %rsp, %rdi\n"        // pass stack pointer as first arg
    "    and $-16, %rsp\n"        // align stack to 16 bytes
    "    call _start_c\n"
    "    ud2\n"                   // should never return
);
