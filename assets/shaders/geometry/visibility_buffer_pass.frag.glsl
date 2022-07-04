#version 460

#extension GL_ARB_gpu_shader_int64 : require
#ifdef USE_SOFTWARE_DEPTH_TEST
#extension GL_EXT_shader_image_int64 : require
#endif

#ifdef USE_SOFTWARE_DEPTH_TEST
layout(set = 0, binding = 0, r64ui) uniform u64image2D visibility_image;
#endif

layout(location = 0) in flat uint cluster_id;

#ifndef USE_SOFTWARE_DEPTH_TEST
layout(location = 0) out uvec2 out_visibility;
#endif

uint64_t encode_visibility(uint depth, uint cluster_id, uint triangle_id) {
    return (uint64_t(depth) & 0x3FFFFFFF) << 34 | (uint64_t(cluster_id) & 0x7FFFFFF) << 7 | uint64_t(triangle_id) & 0x7F;
}

void main() {
    const ivec2 frag_coord = ivec2(gl_FragCoord.xy);

    const uint depth = uint((gl_FragCoord.z / gl_FragCoord.w) * float(0x3FFFFFFF));
    const uint64_t visibility = encode_visibility(depth, cluster_id, gl_PrimitiveID);

#ifdef USE_SOFTWARE_DEPTH_TEST
    imageAtomicMax(visibility_image, frag_coord, visibility);
#else
    out_visibility = uvec2(uint(visibility >> 32), uint(visibility));
#endif
}