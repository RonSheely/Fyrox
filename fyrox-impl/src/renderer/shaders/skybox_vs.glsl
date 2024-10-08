layout (location = 0) in vec3 vertexPosition;

layout (std140) uniform Uniforms {
    mat4 worldViewProjection;
};

out vec3 texCoord;

void main()
{
    texCoord = vertexPosition;
    gl_Position = worldViewProjection * vec4(vertexPosition, 1.0);
}
