#include "process.h"

int get_type(CMD *cmd) { return cmd->type; }
int get_argc(CMD *cmd) { return cmd->argc; }
char** get_argv(CMD *cmd) { return cmd->argv; }
int get_nLocal(CMD *cmd) { return cmd->nLocal; }
char** get_locVar(CMD *cmd) { return cmd->locVar; }
char** get_locVal(CMD *cmd) { return cmd->locVal; }
int get_fromType(CMD *cmd) { return cmd->fromType; }
char* get_fromFile(CMD *cmd) { return cmd->fromFile; }
int get_toType(CMD *cmd) { return cmd->toType; }
char* get_toFile(CMD *cmd) { return cmd->toFile; }
int get_errType(CMD *cmd) { return cmd->errType; }
char* get_errFile(CMD *cmd) { return cmd->errFile; }
struct cmd* get_left(CMD *cmd) { return cmd->left; }
struct cmd* get_right(CMD *cmd) { return cmd->right; }
char* get_cstr(char **array, int idx) { return array[idx]; }
