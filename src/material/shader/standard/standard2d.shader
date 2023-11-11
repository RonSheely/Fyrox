(
    name: "Standard2DShader",

    properties: [
        (
            name: "diffuseTexture",
            kind: Sampler(default: None, fallback: White),
        ),
    ],

    passes: [
        (
            name: "Forward",
            draw_parameters: DrawParameters(
                cull_face: Some(Back),
                color_write: ColorMask(
                    red: true,
                    green: true,
                    blue: true,
                    alpha: true,
                ),
                depth_write: true,
                stencil_test: None,
                depth_test: true,
                blend: Some(BlendParameters(
                    func: BlendFunc(
                        sfactor: SrcAlpha,
                        dfactor: OneMinusSrcAlpha,
                        alpha_sfactor: SrcAlpha,
                        alpha_dfactor: OneMinusSrcAlpha,
                    ),
                    equation: BlendEquation(
                        rgb: Add,
                        alpha: Add
                    )
                )),
                stencil_op: StencilOp(
                    fail: Keep,
                    zfail: Keep,
                    zpass: Keep,
                    write_mask: 0xFFFF_FFFF,
                ),
            ),
            vertex_shader:
               r#"
                layout(location = 0) in vec3 vertexPosition;
                layout(location = 1) in vec2 vertexTexCoord;
                layout(location = 2) in vec4 vertexColor;

                uniform mat4 fyrox_worldViewProjection;
                uniform mat4 fyrox_worldMatrix;

                out vec2 texCoord;
                out vec4 color;
                out vec3 fragmentPosition;

                void main()
                {
                    texCoord = vertexTexCoord;
                    fragmentPosition = (fyrox_worldMatrix * vec4(vertexPosition, 1.0)).xyz;
                    gl_Position = fyrox_worldViewProjection * vec4(vertexPosition, 1.0);
                    color = vertexColor;
                }
               "#,

           fragment_shader:
               r#"
                uniform sampler2D diffuseTexture;

                uniform int lightCount;
                uniform vec4 lightColorRadius[16]; // xyz - color, w = radius
                uniform vec3 lightPosition[16];
                uniform vec3 lightDirection[16];
                uniform vec2 lightParameters[16]; // x = hotspot angle, y - full cone angle delta
                uniform vec3 ambientLightColor;

                out vec4 FragColor;

                in vec2 texCoord;
                in vec4 color;
                in vec3 fragmentPosition;

                void main()
                {
                    vec3 lighting = ambientLightColor;
                    for(int i = 0; i < lightCount; ++i) {
                        // "Unpack" light parameters.
                        float halfHotspotAngleCos = lightParameters[i].x;
                        float halfConeAngleCos = lightParameters[i].y;
                        vec3 lightColor = lightColorRadius[i].xyz;
                        float radius = lightColorRadius[i].w;
                        vec3 lightPosition = lightPosition[i];
                        vec3 direction = lightDirection[i];

                        // Calculate lighting.
                        vec3 toFragment = fragmentPosition - lightPosition;
                        float distance = length(toFragment);
                        vec3 toFragmentNormalized = toFragment / distance;
                        float distanceAttenuation = S_LightDistanceAttenuation(distance, radius);
                        float spotAngleCos = dot(toFragmentNormalized, direction);
                        float directionalAttenuation = smoothstep(halfConeAngleCos, halfHotspotAngleCos, spotAngleCos);
                        lighting += lightColor * (distanceAttenuation * directionalAttenuation);
                    }

                    FragColor = vec4(lighting, 1.0) * color * S_SRGBToLinear(texture(diffuseTexture, texCoord));
                }
               "#,
        )
    ],
)