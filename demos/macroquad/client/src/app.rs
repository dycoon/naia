use std::{ops::Deref, collections::HashSet};

use macroquad::prelude::*;

use naia_client::{Client as NaiaClient, ClientConfig, Event, Protocolize};

use naia_demo_world::{Entity, World as DemoWorld, WorldMutType, WorldRefType};

use naia_macroquad_demo_shared::{
    behavior as shared_behavior, get_server_address, get_shared_config,
    protocol::{Auth, Color, KeyCommand, Protocol, Square},
};

type World = DemoWorld<Protocol>;
type Client = NaiaClient<Protocol, Entity>;

const SQUARE_SIZE: f32 = 32.0;

struct OwnedEntity {
    pub confirmed: Entity,
    pub predicted: Entity,
}

impl OwnedEntity {
    pub fn new(confirmed_entity: Entity, predicted_entity: Entity) -> Self {
        OwnedEntity { confirmed: confirmed_entity, predicted: predicted_entity }
    }
}

pub struct App {
    client: Client,
    world: World,
    queued_command: Option<KeyCommand>,
    owned_entity: Option<OwnedEntity>,
    squares: HashSet<Entity>,
}

impl App {
    pub fn new() -> Self {
        info!("Naia Macroquad Client Demo started");

        let server_address = get_server_address();
        let auth = Auth::new("charlie", "12345");

        let mut client = Client::new(ClientConfig::default(), get_shared_config());
        client.auth(auth);
        client.connect(server_address);

        App {
            client,
            world: World::new(),
            queued_command: None,
            owned_entity: None,
            squares: HashSet::new(),
        }
    }

    pub fn update(&mut self) {
        self.input();
        self.receive_events();
        self.draw();
    }

    fn input(&mut self) {
        if let Some(_) = self.owned_entity {
            let w = is_key_down(KeyCode::W);
            let s = is_key_down(KeyCode::S);
            let a = is_key_down(KeyCode::A);
            let d = is_key_down(KeyCode::D);

            if let Some(command) = &mut self.queued_command {
                if w {
                    command.w.set(true);
                }
                if s {
                    command.s.set(true);
                }
                if a {
                    command.a.set(true);
                }
                if d {
                    command.d.set(true);
                }
            } else {
                self.queued_command = Some(KeyCommand::new(w, s, a, d));
            }
        }
    }

    fn receive_events(&mut self) {
        for event in self.client.receive(self.world.proxy_mut()) {
            match event {
                Ok(Event::Connection) => {
                    info!("Client connected to: {}", self.client.server_address());
                }
                Ok(Event::Disconnection) => {
                    info!("Client disconnected from: {}", self.client.server_address());
                }
                Ok(Event::Tick) => {
                    if let Some(owned_entity) = &self.owned_entity {
                        if let Some(command) = self.queued_command.take() {

                            // Send command
                            self.client.entity_mut(&owned_entity.confirmed).send_message(&command);

                            // Apply command
                            if let Some(mut square_ref) =
                            self.world.proxy_mut().get_component_mut::<Square>(&owned_entity.predicted)
                            {
                                shared_behavior::process_command(&command, &mut square_ref);
                            }
                        }
                    }
                }
                Ok(Event::SpawnEntity(entity, _)) => {
                    self.squares.insert(entity);
                }
                Ok(Event::DespawnEntity(entity)) => {
                    self.squares.remove(&entity);
                }
                Ok(Event::MessageEntity(entity, Protocol::EntityAssignment(entity_assignment))) => {
                    let assign = *entity_assignment.assign.get();

                    if assign {
                        info!("gave ownership of entity");

                        ////////////////////////////////
                        let mut world_mut = self.world.proxy_mut();
                        let prediction_entity = world_mut.spawn_entity();

                        // create copies of components //
                        for component_kind in world_mut.get_component_kinds(&entity) {
                            let mut component_copy_opt: Option<Protocol> = None;
                            if let Some(component) =
                                world_mut.get_component_of_kind(&entity, &component_kind)
                            {
                                component_copy_opt =
                                    Some(component.deref().deref().protocol_copy());
                            }
                            if let Some(component_copy) = component_copy_opt {
                                component_copy.extract_and_insert(&prediction_entity, &mut world_mut);
                            }
                        }
                        ////////////////////////////////

                        self.owned_entity = Some(OwnedEntity::new(entity, prediction_entity));
                    } else {
                        let mut disowned: bool = false;
                        if let Some(owned_entity) = &self.owned_entity {
                            if owned_entity.confirmed == entity {
                                self.world.proxy_mut().despawn_entity(&owned_entity.predicted);
                                disowned = true;
                            }
                        }
                        if disowned {
                            info!("removed ownership of entity");
                            self.owned_entity = None;
                        }
                    }
                }
                Err(err) => {
                    info!("Client Error: {}", err);
                }
                _ => {}
            }
        }
    }

    fn draw(&mut self) {
        clear_background(BLACK);

        if self.client.connected() {
            // draw unowned squares
            for entity in &self.squares {
                if let Some(square) = self.world.proxy().get_component::<Square>(entity) {
                    let color = match square.color.get() {
                        Color::Red => RED,
                        Color::Blue => BLUE,
                        Color::Yellow => YELLOW,
                    };
                    draw_rectangle(
                        f32::from(*(square.x.get())),
                        f32::from(*(square.y.get())),
                        SQUARE_SIZE,
                        SQUARE_SIZE,
                        color,
                    );
                }
            }

            // draw own square
            if let Some(entity) = &self.owned_entity {
                if let Some(square) = self.world.proxy().get_component::<Square>(&entity.predicted) {
                    draw_rectangle(
                        f32::from(*(square.x.get())),
                        f32::from(*(square.y.get())),
                        SQUARE_SIZE,
                        SQUARE_SIZE,
                        WHITE,
                    );
                }
            }
        }
    }
}
