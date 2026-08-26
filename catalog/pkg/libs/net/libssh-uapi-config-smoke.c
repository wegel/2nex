/*
 * libssh UAPI configuration smoke test.
 *
 * Exercises the newly built, installed libssh through its real readers
 * rather than a copied algorithm:
 *
 *   - "client-config" calls ssh_options_parse_config(session, NULL), the
 *     same entry point ssh_connect() calls automatically, and reads back
 *     the resolved ProxyCommand with ssh_options_get().
 *   - "bind-listen" calls ssh_bind_listen(), which itself calls
 *     ssh_bind_options_parse_config(sshbind, NULL) before it binds, and
 *     reports the port the kernel actually bound.
 *   - "moduli" drives a real diffie-hellman-group-exchange handshake
 *     between an in-process client and server connected by a socketpair,
 *     and reports whether the handshake completed. The moduli reader
 *     (ssh_retrieve_dhgroup()) is static and not exported, so a full
 *     handshake is the cheapest way to exercise it through the real
 *     library: a system tier that exists but is empty has zero usable
 *     entries, which makes the server fail the DH group exchange instead
 *     of silently falling back, and a missing tier is skipped instead.
 *
 * Every libssh symbol this file calls is exported; this was checked with
 * `nm -D` on the freshly built libssh.so before wiring this file into the
 * package build (see libssh-uapi-config-check.sh).
 */

#include <netinet/in.h>
#include <stdbool.h>
#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>

#include <libssh/libssh.h>
#include <libssh/server.h>

static int
do_client_config(void)
{
    ssh_session session;
    char *value = NULL;
    int rc;

    session = ssh_new();
    if (session == NULL) {
        fprintf(stderr, "ssh_new failed\n");
        return 2;
    }

    if (ssh_options_set(session, SSH_OPTIONS_HOST,
                        "libssh-uapi-smoke.invalid") != 0) {
        fprintf(stderr, "ssh_options_set(HOST) failed\n");
        ssh_free(session);
        return 2;
    }

    rc = ssh_options_parse_config(session, NULL);
    if (rc != 0) {
        fprintf(stderr, "ssh_options_parse_config failed: %s\n",
                ssh_get_error(session));
        ssh_free(session);
        return 2;
    }

    rc = ssh_options_get(session, SSH_OPTIONS_PROXYCOMMAND, &value);
    if (rc == SSH_OK && value != NULL) {
        printf("%s\n", value);
        ssh_string_free_char(value);
    } else {
        printf("NONE\n");
    }

    ssh_free(session);
    return 0;
}

static int
do_bind_listen(const char *preset_port)
{
    ssh_bind sshbind;
    ssh_key key = NULL;
    struct sockaddr_storage addr;
    socklen_t addr_len = sizeof(addr);
    socket_t fd;
    unsigned int bound_port = 0;
    int rc;

    sshbind = ssh_bind_new();
    if (sshbind == NULL) {
        fprintf(stderr, "ssh_bind_new failed\n");
        return 2;
    }

    /* A private, freshly generated host key: never touch the machine's own
     * /etc/ssh/ssh_host_* secrets, and never require them to exist. */
    rc = ssh_pki_generate(SSH_KEYTYPE_ED25519, 256, &key);
    if (rc != SSH_OK) {
        fprintf(stderr, "ssh_pki_generate failed\n");
        ssh_bind_free(sshbind);
        return 2;
    }
    rc = ssh_bind_options_set(sshbind, SSH_BIND_OPTIONS_IMPORT_KEY, key);
    if (rc != 0) {
        fprintf(stderr, "ssh_bind_options_set(IMPORT_KEY) failed: %s\n",
                ssh_get_error(sshbind));
        ssh_key_free(key);
        ssh_bind_free(sshbind);
        return 2;
    }

    rc = ssh_bind_options_set(sshbind, SSH_BIND_OPTIONS_BINDADDR,
                              "127.0.0.1");
    if (rc != 0) {
        fprintf(stderr, "ssh_bind_options_set(BINDADDR) failed\n");
        ssh_bind_free(sshbind);
        return 2;
    }

    if (preset_port != NULL && strcmp(preset_port, "-") != 0) {
        rc = ssh_bind_options_set(sshbind, SSH_BIND_OPTIONS_BINDPORT_STR,
                                  preset_port);
        if (rc != 0) {
            fprintf(stderr, "ssh_bind_options_set(BINDPORT_STR) failed\n");
            ssh_bind_free(sshbind);
            return 2;
        }
    }

    /* ssh_bind_listen() calls ssh_bind_options_parse_config(sshbind, NULL)
     * itself before it binds; this is the same public entry point a real
     * sshd-style server calls. */
    rc = ssh_bind_listen(sshbind);
    if (rc != SSH_OK) {
        printf("LISTEN_FAILED\n");
        ssh_bind_free(sshbind);
        return 0;
    }

    fd = ssh_bind_get_fd(sshbind);
    if (getsockname(fd, (struct sockaddr *)&addr, &addr_len) != 0) {
        fprintf(stderr, "getsockname failed\n");
        ssh_bind_free(sshbind);
        return 2;
    }
    if (addr.ss_family == AF_INET) {
        bound_port = ntohs(((struct sockaddr_in *)&addr)->sin_port);
    } else if (addr.ss_family == AF_INET6) {
        bound_port = ntohs(((struct sockaddr_in6 *)&addr)->sin6_port);
    }

    printf("%u\n", bound_port);
    ssh_bind_free(sshbind);
    return 0;
}

static void
moduli_server(int fd)
{
    ssh_bind sshbind;
    ssh_session session;
    ssh_key key = NULL;

    sshbind = ssh_bind_new();
    session = ssh_new();
    if (sshbind == NULL || session == NULL) {
        _exit(2);
    }

    if (ssh_pki_generate(SSH_KEYTYPE_ED25519, 256, &key) != SSH_OK) {
        _exit(2);
    }
    if (ssh_bind_options_set(sshbind, SSH_BIND_OPTIONS_IMPORT_KEY, key) !=
        0) {
        _exit(2);
    }

    /* ssh_bind_accept_fd() never calls ssh_bind_options_parse_config()
     * (only ssh_bind_listen() does), so this server never reads the bind
     * config; the moduli reader this test exercises is independent of it. */
    if (ssh_bind_accept_fd(sshbind, session, fd) != SSH_OK) {
        _exit(2);
    }

    /* The client only offers diffie-hellman-group-exchange-sha256 (see
     * moduli_client()), so a successful key exchange here can only have
     * gone through ssh_retrieve_dhgroup() and therefore through the moduli
     * tier selection this patch changes. */
    (void)ssh_handle_key_exchange(session);

    /* The client alone decides pass/fail for this test by observing
     * whether ssh_connect() completed; the server just needs to run the
     * real reader and exit. */
    _exit(0);
}

static int
do_moduli(void)
{
    int fds[2];
    pid_t pid;
    ssh_session session;
    bool no = false;
    long timeout = 3;
    int rc;
    int status;

    if (socketpair(AF_UNIX, SOCK_STREAM, 0, fds) != 0) {
        fprintf(stderr, "socketpair failed\n");
        return 2;
    }

    pid = fork();
    if (pid < 0) {
        fprintf(stderr, "fork failed\n");
        return 2;
    }
    if (pid == 0) {
        close(fds[0]);
        moduli_server(fds[1]);
        _exit(2); /* unreachable */
    }
    close(fds[1]);

    session = ssh_new();
    if (session == NULL) {
        fprintf(stderr, "ssh_new failed\n");
        return 2;
    }
    ssh_options_set(session, SSH_OPTIONS_HOST, "localhost");
    /* ssh_connect() calls ssh_options_apply(), which resolves a default
     * username through the system user database when none is set. The
     * package build sandbox has no matching passwd entry for the build
     * user, so set one explicitly instead of relying on that lookup. */
    ssh_options_set(session, SSH_OPTIONS_USER, "libssh-uapi-smoke");
    ssh_options_set(session, SSH_OPTIONS_FD, &fds[0]);
    ssh_options_set(session, SSH_OPTIONS_PROCESS_CONFIG, &no);
    ssh_options_set(session, SSH_OPTIONS_TIMEOUT, &timeout);
    ssh_options_set(session, SSH_OPTIONS_KEY_EXCHANGE,
                    "diffie-hellman-group-exchange-sha256");

    rc = ssh_connect(session);
    printf("%s\n", rc == SSH_OK ? "CONNECT_OK" : "CONNECT_FAIL");

    ssh_disconnect(session);
    ssh_free(session);

    /* SSH_OPTIONS_FD makes fds[0] caller-owned, so ssh_free() never closes
     * it. Leaving it open would keep the socketpair connected from the
     * server's side and hang moduli_server()'s blocking read forever. */
    close(fds[0]);

    waitpid(pid, &status, 0);
    return 0;
}

int
main(int argc, char **argv)
{
    if (argc < 2) {
        fprintf(stderr,
                "usage: %s client-config | bind-listen PORT | moduli\n",
                argv[0]);
        return 2;
    }

    if (strcmp(argv[1], "client-config") == 0) {
        return do_client_config();
    }
    if (strcmp(argv[1], "bind-listen") == 0) {
        return do_bind_listen(argc > 2 ? argv[2] : NULL);
    }
    if (strcmp(argv[1], "moduli") == 0) {
        return do_moduli();
    }

    fprintf(stderr, "unknown mode: %s\n", argv[1]);
    return 2;
}
