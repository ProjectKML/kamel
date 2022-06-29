#version 460

#extension GL_NV_mesh_shader : enable

layout(local_size_x = 32) in;
layout(triangles, max_vertices = 64, max_primitives = 126) out;

struct VisibleCluster {
    uint instance_id;
    uint cluster_id;
};

layout(set = 0, binding = 0) readonly buffer VisibleClusters {
    VisibleCluster[] visible_clusters;
};

layout(location = 0) out flat uint[] cluster_ids;

void main() {

}