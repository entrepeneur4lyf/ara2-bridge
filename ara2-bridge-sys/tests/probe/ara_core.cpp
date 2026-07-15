// C++ parity probe for declarations missing from the released C branch.
#include "ARAAudioFileChunks.h"

extern "C" const char *ara_cpp_create_distinct_audio_modification(void)
{
    return ARA::kARAXMLName_CreateDistinctAudioModification;
}
