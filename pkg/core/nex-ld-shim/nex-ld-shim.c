/*
 * nex-ld-shim.c - Dynamic linker shim for 2nex
 *
 * This shim is invoked as the PT_INTERP for all 2nex-built binaries.
 * It locates the app root, finds the correct loader and library directory,
 * then execs the real glibc loader in CLI mode with proper arguments.
 *
 * Requirements:
 * - glibc >= 2.33 (for --argv0 support)
 * - Static linking (no dynamic dependencies)
 *
 * Security: This is part of the trusted bootstrap path. Setuid/setgid
 * execution is not supported.
 */

#define _GNU_SOURCE

// disable false positive format-truncation warnings
#pragma GCC diagnostic ignored "-Wformat-truncation"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <limits.h>
#include <errno.h>
#include <sys/auxv.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <libgen.h>

// error reporting: use write() to avoid stdio overhead
static void fatal_error(const char *msg) {
    const char prefix[] = "nex-ld-shim: error: ";
    write(2, prefix, sizeof(prefix) - 1);
    write(2, msg, strlen(msg));
    write(2, "\n", 1);
    _exit(1);
}

static void fatal_error_with_path(const char *msg, const char *path) {
    const char prefix[] = "nex-ld-shim: error: ";
    write(2, prefix, sizeof(prefix) - 1);
    write(2, msg, strlen(msg));
    write(2, ": ", 2);
    write(2, path, strlen(path));
    write(2, "\n", 1);
    _exit(1);
}

static void fatal_error_errno(const char *msg) {
    const char prefix[] = "nex-ld-shim: error: ";
    char buf[256];
    write(2, prefix, sizeof(prefix) - 1);
    write(2, msg, strlen(msg));
    write(2, ": ", 2);
    snprintf(buf, sizeof(buf), "%s", strerror(errno));
    write(2, buf, strlen(buf));
    write(2, "\n", 1);
    _exit(1);
}

// check if a file exists and is a regular file
static int file_exists(const char *path) {
    struct stat st;
    if (stat(path, &st) == 0 && S_ISREG(st.st_mode)) {
        return 1;
    }
    return 0;
}

// walk up directory tree to find .nex-app-root
// returns 1 on success, 0 on failure
static int find_app_root(const char *resolved_exe, char *out_root, size_t out_sz) {
    char path[PATH_MAX];
    char *dir;

    // start with the directory containing the executable
    strncpy(path, resolved_exe, sizeof(path) - 1);
    path[sizeof(path) - 1] = '\0';

    dir = dirname(path);

    while (1) {
        char sentinel[PATH_MAX];

        // check for .nex-app-root in current directory
        snprintf(sentinel, sizeof(sentinel), "%s/.nex-app-root", dir);

        if (file_exists(sentinel)) {
            // found it!
            strncpy(out_root, dir, out_sz - 1);
            out_root[out_sz - 1] = '\0';
            return 1;
        }

        // if we're at root, stop
        if (strcmp(dir, "/") == 0) {
            break;
        }

        // move up one directory
        strncpy(path, dir, sizeof(path) - 1);
        path[sizeof(path) - 1] = '\0';
        dir = dirname(path);
    }

    return 0;
}

// pick the loader and lib directory
// returns 1 on success, 0 on failure
static int pick_loader_and_lib(const char *app_root, char *loader_path, char *lib_dir,
                                size_t loader_sz, size_t lib_sz) {
    char nex_loader_file[PATH_MAX * 2];

    // set lib_dir to app_root/lib
    snprintf(lib_dir, lib_sz, "%s/lib", app_root);

    // check for nex.loader override file
    snprintf(nex_loader_file, sizeof(nex_loader_file), "%s/nex.loader", app_root);

    if (file_exists(nex_loader_file)) {
        // read first line from nex.loader
        FILE *f = fopen(nex_loader_file, "r");
        if (!f) {
            fatal_error_with_path("could not open nex.loader", nex_loader_file);
        }

        if (fgets(loader_path, loader_sz, f) == NULL) {
            fclose(f);
            fatal_error_with_path("could not read nex.loader", nex_loader_file);
        }

        fclose(f);

        // trim newline
        size_t len = strlen(loader_path);
        if (len > 0 && loader_path[len - 1] == '\n') {
            loader_path[len - 1] = '\0';
        }
    } else {
        // use default: lib_dir/ld-linux-x86-64.so.2
        snprintf(loader_path, loader_sz, "%s/ld-linux-x86-64.so.2", lib_dir);
    }

    // verify loader exists
    if (!file_exists(loader_path)) {
        fatal_error_with_path("loader not found", loader_path);
    }

    return 1;
}

// build argv for the loader
// NOTE: returns pointer to caller-allocated buffer
static void build_loader_argv(const char *loader_path, const char *orig_argv0,
                               const char *lib_dir, const char *resolved_exe,
                               int argc, char **argv, char **out_argv,
                               char *loader_str, char *lib_str, char *exe_str) {
    // we need: loader_path --argv0 orig_argv0 --library-path lib_dir resolved_exe arg1 arg2 ...
    // copy strings to caller-provided buffers to avoid strdup
    strcpy(loader_str, loader_path);
    strcpy(lib_str, lib_dir);
    strcpy(exe_str, resolved_exe);

    int idx = 0;
    out_argv[idx++] = loader_str;
    out_argv[idx++] = "--argv0";
    out_argv[idx++] = (char *)orig_argv0;
    out_argv[idx++] = "--library-path";
    out_argv[idx++] = lib_str;
    out_argv[idx++] = exe_str;

    // copy remaining args from original argv (skip argv[0])
    for (int i = 1; i < argc; i++) {
        out_argv[idx++] = argv[i];
    }

    out_argv[idx] = NULL;
}

// check if a string is in the environment
static int env_has_prefix(const char *env_entry, const char *prefix) {
    size_t prefix_len = strlen(prefix);
    if (strncmp(env_entry, prefix, prefix_len) == 0 && env_entry[prefix_len] == '=') {
        return 1;
    }
    return 0;
}

// build envp for the loader
// NOTE: uses caller-allocated buffers
static void build_loader_env(char **envp, const char *app_root, const char *lib_dir,
                              char **out_envp, char *app_root_str, char *lib_dir_str) {
    int idx = 0;

    // copy existing env, but skip LD_LIBRARY_PATH, LD_PRELOAD, etc for security
    for (char **e = envp; *e != NULL; e++) {
        if (env_has_prefix(*e, "LD_LIBRARY_PATH") ||
            env_has_prefix(*e, "LD_PRELOAD") ||
            env_has_prefix(*e, "LD_AUDIT")) {
            // skip these for security
            continue;
        }
        out_envp[idx++] = *e;
    }

    // add NEX_APP_ROOT
    snprintf(app_root_str, PATH_MAX * 2, "NEX_APP_ROOT=%s", app_root);
    out_envp[idx++] = app_root_str;

    // optionally add NEX_LIB_DIR for debugging
    snprintf(lib_dir_str, PATH_MAX * 2, "NEX_LIB_DIR=%s", lib_dir);
    out_envp[idx++] = lib_dir_str;

    out_envp[idx] = NULL;
}

int main(int argc, char **argv, char **envp) {
    // security check: don't run in secure mode (setuid/setgid)
    unsigned long at_secure = getauxval(AT_SECURE);
    if (at_secure != 0) {
        fatal_error("execution in secure mode (setuid/setgid) not supported");
    }

    // 1. get the path to the main executable via AT_EXECFN
    const char *execfn = (const char *)getauxval(AT_EXECFN);
    if (!execfn) {
        fatal_error("AT_EXECFN not available");
    }

    // 2. canonicalize the path
    char resolved_exe[PATH_MAX];
    if (realpath(execfn, resolved_exe) == NULL) {
        // fallback: try /proc/self/exe
        ssize_t len = readlink("/proc/self/exe", resolved_exe, sizeof(resolved_exe) - 1);
        if (len < 0) {
            fatal_error_errno("could not resolve executable path");
        }
        resolved_exe[len] = '\0';
    }

    // 3. determine original argv[0]
    const char *orig_argv0 = argv[0];

    // 4. find app root
    char app_root[PATH_MAX];
    if (!find_app_root(resolved_exe, app_root, sizeof(app_root))) {
        fatal_error_with_path(".nex-app-root not found for", resolved_exe);
    }

    // 5. pick loader and lib dir
    char loader_path[PATH_MAX * 2];
    char lib_dir[PATH_MAX * 2];
    if (!pick_loader_and_lib(app_root, loader_path, lib_dir,
                             sizeof(loader_path), sizeof(lib_dir))) {
        fatal_error("could not determine loader");
    }

    // 6. construct new argv (stack allocated)
    // count env vars for sizing
    int env_count = 0;
    for (char **e = envp; *e != NULL; e++) {
        env_count++;
    }

    // allocate on stack using VLA
    char *new_argv[argc + 6];
    char *new_envp[env_count + 3];

    // string buffers for paths
    char loader_str[PATH_MAX * 2];
    char lib_str[PATH_MAX * 2];
    char exe_str[PATH_MAX];
    char app_root_env[PATH_MAX * 2];
    char lib_dir_env[PATH_MAX * 2];

    build_loader_argv(loader_path, orig_argv0, lib_dir, resolved_exe, argc, argv,
                      new_argv, loader_str, lib_str, exe_str);

    // 7. construct new envp
    build_loader_env(envp, app_root, lib_dir, new_envp, app_root_env, lib_dir_env);

    // 8. exec the real loader
    execve(loader_path, new_argv, new_envp);

    // if we get here, execve failed
    fatal_error_errno("execve failed");

    return 1; // never reached
}
