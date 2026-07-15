// Core ARA C ABI probe driver. The generated table is derived from symbol coverage.
#include <stddef.h>
#include <stdio.h>

#include "ARAInterface.h"
#include "ara_probe_table.inc"

extern const char *ara_cpp_create_distinct_audio_modification(void);

int main(void)
{
    fputs("{\"structs\":{", stdout);
    ara_probe_emit_structs();
    fputs("},\"constants\":{", stdout);
    ara_probe_emit_constants();
    fputs("},\"cpp\":{\"kARAXMLName_CreateDistinctAudioModification\":\"", stdout);
    fputs(ara_cpp_create_distinct_audio_modification(), stdout);
    fputs("\"}}", stdout);
    return ferror(stdout) ? 1 : 0;
}
