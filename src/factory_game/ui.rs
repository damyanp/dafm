use bevy::prelude::*;

use crate::{GameState, factory_game::interaction::Tools};

pub struct UiPlugin;
impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::FactoryGame), create_ui)
            .add_systems(
                Update,
                update_tools.run_if(resource_exists_and_changed::<Tools>),
            );
    }
}

fn create_ui(mut commands: Commands) {
    let tools: Vec<_> = (0..11usize)
        .map(|_| {
            commands
                .spawn((
                    StateScoped(GameState::FactoryGame),
                    Node {
                        margin: UiRect::all(Val::Px(2.0)),
                        min_width: Val::Px(32.0),
                        min_height: Val::Px(32.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.7, 0.7, 0.7)),
                ))
                .id()
        })
        .collect();

    commands
        .spawn((
            StateScoped(GameState::FactoryGame),
            ToolsPanel,
            Node {
                margin: UiRect::all(Val::Px(10.0)),
                align_self: AlignSelf::End,
                justify_self: JustifySelf::Center,
                justify_content: JustifyContent::SpaceEvenly,
                min_width: Val::Px(32.0),
                min_height: Val::Px(5.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.7, 0.7, 0.7, 0.5)),
        ))
        .add_children(&tools);
}

#[derive(Component)]
struct ToolsPanel;

fn update_tools(
    mut commands: Commands,
    tools: Res<Tools>,
    panel: Single<Entity, With<ToolsPanel>>,
) {
    if tools.is_added() {
        let tool_selectors: Vec<_> = tools
            .tools()
            .iter()
            .map(|t| {
                commands
                    .spawn((
                        StateScoped(GameState::FactoryGame),
                        Node {
                            margin: UiRect::all(Val::Px(2.0)),
                            min_width: Val::Px(32.0),
                            min_height: Val::Px(32.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.7, 0.7, 0.7)),
                    ))
                    .with_children(|c| {
                        c.spawn(Text::new(format!("{}", t.slot())));
                    })
                    .id()
            })
            .collect();

        commands
            .entity(*panel)
            .despawn_related::<Children>()
            .add_children(&tool_selectors);
    }
}
