use bevy::prelude::*;

use crate::{GameState, factory_game::interaction::Tools, sprite_sheet::SpriteSheet};

pub fn ui_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::FactoryGame), create_ui)
        .add_systems(
            Update,
            update_tools.run_if(resource_exists_and_changed::<Tools>),
        );
}

fn create_ui(mut commands: Commands) {
    commands.spawn((
        StateScoped(GameState::FactoryGame),
        ToolsPanel,
        Node {
            margin: UiRect::all(Val::Px(10.0)),
            border: UiRect::all(Val::Px(1.0)),
            align_self: AlignSelf::End,
            justify_self: JustifySelf::Center,
            justify_content: JustifyContent::SpaceEvenly,
            min_width: Val::Px(32.0),
            min_height: Val::Px(5.0),
            ..default()
        },
        BackgroundColor(Color::BLACK),
        BorderColor(Color::WHITE),
    ));
}

#[derive(Component)]
struct ToolsPanel;

#[derive(Component)]
struct ToolPanelSlot(u32);

fn update_tools(
    mut commands: Commands,
    tools: Res<Tools>,
    sprite_sheet: Res<SpriteSheet>,
    panel: Single<Entity, With<ToolsPanel>>,
    tool_panel_slots: Query<(&mut BackgroundColor, &mut Node, &ToolPanelSlot)>,
) {
    let inactive_color = Color::srgb(0.4, 0.4, 0.4);
    let active_color = Color::srgb(0.4, 0.1, 0.1);

    let is_active_slot = |slot| {
        tools
            .current_tool()
            .map(|entry| entry.slot() == slot)
            .unwrap_or(false)
    };

    if tools.is_added() {
        let tool_selectors: Vec<_> = tools
            .tools()
            .iter()
            .map(|t| {
                commands
                    .spawn((
                        ToolPanelSlot(t.slot()),
                        Node {
                            margin: UiRect::all(Val::Px(2.0)),
                            padding: UiRect::all(Val::Px(10.0)),
                            min_width: Val::Px(32.0),
                            min_height: Val::Px(32.0),
                            ..default()
                        },
                        BackgroundColor(inactive_color),
                    ))
                    .with_children(|c| {
                        let (sprite, _) = t.tool().get_sprite_flip();

                        c.spawn((
                            ImageNode::from_atlas_image(
                                sprite_sheet.image(),
                                sprite_sheet.texture_atlas(sprite),
                            ),
                            Node {
                                width: Val::Px(32.0),
                                height: Val::Px(32.0),
                                ..default()
                            },
                        ));
                        c.spawn((
                            Text::new(format!("{}", t.slot())),
                            TextFont::from_font_size(14.0),
                            Node {
                                position_type: PositionType::Absolute,
                                align_self: AlignSelf::End,
                                ..default()
                            },
                        ));
                    })
                    .id()
            })
            .collect();

        commands
            .entity(*panel)
            .despawn_related::<Children>()
            .add_children(&tool_selectors);
        return;
    }

    for (mut bg, mut node, slot) in tool_panel_slots {
        if is_active_slot(slot.0) {
            bg.0 = active_color;
            node.top = Val::Px(-2.0);
        } else {
            bg.0 = inactive_color;
            node.top = Val::Auto;
        }
    }
}
