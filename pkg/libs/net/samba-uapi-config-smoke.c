/*
 * samba (libsmbclient) UAPI configuration smoke test.
 *
 * Exercises the newly built, installed libsmbclient through its real system
 * config reader rather than a copied algorithm: smbc_new_context() runs the
 * module-wide smb.conf load (SMBC_module_init(), guarded by a one-time
 * pthread_once in this process), then smbc_init_context() resolves the
 * effective "workgroup" parameter through lp_workgroup() and stores it on
 * the context. smbc_getWorkgroup() reads that resolved value back.
 *
 * Every libsmbclient symbol this file calls is exported; this was checked
 * with `nm -D` on the freshly built libsmbclient.so before wiring this file
 * into the package build (see samba-uapi-config-check.sh).
 */

#include <stdio.h>

#include <libsmbclient.h>

int
main(void)
{
    SMBCCTX *ctx;
    const char *workgroup;

    ctx = smbc_new_context();
    if (ctx == NULL) {
        fprintf(stderr, "smbc_new_context failed\n");
        return 2;
    }

    if (smbc_init_context(ctx) == NULL) {
        fprintf(stderr, "smbc_init_context failed\n");
        smbc_free_context(ctx, 1);
        return 2;
    }

    workgroup = smbc_getWorkgroup(ctx);
    printf("%s\n", workgroup != NULL ? workgroup : "NONE");

    smbc_free_context(ctx, 1);
    return 0;
}
