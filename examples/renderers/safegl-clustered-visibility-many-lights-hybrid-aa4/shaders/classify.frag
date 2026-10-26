#version 460 core
// Runs once per pixel and per mesh. Surviving pixels get stencil 1 and are shaded
// per sample; the rest keep stencil 0 and are shaded once for all four samples.
uniform usampler2DMS visibility_texture;
uniform usamplerBuffer index_lookup;
uniform uint triangle_base;
uniform uint triangle_count;

const uint TRIANGLE_MASK = 0x3fffffffu;
const uint ALPHA_COVERAGE = 0x40000000u;
const uint BACK_FACING = 0x80000000u;

bool in_mesh(uint id) {
    uint triangle = (id & TRIANGLE_MASK) - 1u;
    return triangle >= triangle_base && triangle - triangle_base < triangle_count;
}

uvec3 corners(uint id) {
    int first = int(((id & TRIANGLE_MASK) - 1u - triangle_base) * 3u);
    return uvec3(texelFetch(index_lookup, first).r,
                 texelFetch(index_lookup, first + 1).r,
                 texelFetch(index_lookup, first + 2).r);
}

void main() {
    ivec2 pixel = ivec2(gl_FragCoord.xy);
    int samples = textureSamples(visibility_texture);
    uint ids[4];
    bool uniform_id = true, alpha = false, background = false;
    for (int i = 0; i < samples; ++i) {
        ids[i] = texelFetch(visibility_texture, pixel, i).r;
        uniform_id = uniform_id && ids[i] == ids[0];
        alpha = alpha || (ids[i] & ALPHA_COVERAGE) != 0u;
        background = background || (ids[i] & TRIANGLE_MASK) == 0u;
    }
    // Alpha-tested and screen-door materials alias inside triangles, where MSAA
    // coverage cannot help; a mix of geometry and background is a silhouette.
    if (alpha || (background && !uniform_id)) return;
    if (uniform_id) discard;
    // Samples on different triangles: the mesh owning sample 0 decides.
    if (!in_mesh(ids[0])) discard;
    uvec3 owner = corners(ids[0]);
    for (int i = 1; i < samples; ++i) {
        if (ids[i] == ids[0]) continue;
        // Facing flips at silhouettes of double-sided surfaces; other meshes
        // are separate surfaces.
        if (((ids[i] ^ ids[0]) & BACK_FACING) != 0u || !in_mesh(ids[i])) return;
        // Indexed neighbours that share a vertex share its position, normal, and
        // UV, so shading is continuous across them and one centre sample suffices.
        // Creases and UV seams split vertices, so they still count as edges.
        uvec3 other = corners(ids[i]);
        bool adjacent = any(equal(owner, other.xxx)) || any(equal(owner, other.yyy)) ||
                        any(equal(owner, other.zzz));
        if (!adjacent) return;
    }
    discard;
}
