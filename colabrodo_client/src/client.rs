use std::collections::HashMap;

use crate::components::*;
use colabrodo_common::nooid::NooID;

pub trait ComponentList<State> {
    fn on_create(&mut self, id: NooID, state: State);
    fn on_delete(&mut self, id: NooID);
}

pub trait UpdatableComponentList<State>: ComponentList<State> {
    type UpdatePart;
    fn on_update(&mut self, id: NooID, update: Self::UpdatePart);
}

// impl<State> ComponentList<State> {
//     fn insert(
//         &mut self,
//         tid: ComponentType,
//         id: NooID,
//         values: NooValueMap,
//     ) -> Option<()> {
//         let mut converted_map = convert(values);

//         converted_map
//             .entry("name".to_string())
//             .or_insert(Value::Text(format!("{tid} {id}")));

//         let ret = self.components.insert(id, converted_map);
//         if ret.is_some() {
//             // we clobbered a key
//             warn!("Overwrote a component. This could be bad.")
//         }

//         Some(())
//     }
//     fn delete(&mut self, id: NooID, _: NooValueMap) -> Option<()> {
//         let ret = self.components.remove(&id);
//         if ret.is_none() {
//             error!("Asked to delete a component that does not exist!");
//             return None;
//         }
//         Some(())
//     }
//     fn clear(&mut self) {
//         self.components.clear();
//     }

//     fn find(&self, id: NooID) -> Option<&NooHashMap> {
//         self.components.get(&id)
//     }
// }

pub struct DefaultComponentList<State> {
    components: HashMap<NooID, State>,
}

impl<State> ComponentList<State> for DefaultComponentList<State> {
    fn on_create(&mut self, id: NooID, state: State) {
        self.components.insert(id, state);
    }

    fn on_delete(&mut self, id: NooID) {
        self.components.remove(&id);
    }
}

pub trait UserClientState {
    type MethodL: ComponentList<MethodState>;
    type SignalL: ComponentList<SignalState>;

    type BufferL: ComponentList<BufferState>;
    type BufferViewL: ComponentList<ClientBufferViewState>;

    type SamplerL: ComponentList<SamplerState>;
    type ImageL: ComponentList<ClientImageState>;
    type TextureL: ComponentList<ClientTextureState>;

    type MaterialL: ComponentList<ClientMaterialState>;
    type GeometryL: ComponentList<ClientGeometryState>;

    type TableL: UpdatableComponentList<ClientTableState>;
    type PlotL: UpdatableComponentList<ClientPlotState>;

    type EntityL: UpdatableComponentList<ClientEntityState>;

    fn method_list(&mut self) -> &mut Self::MethodL;
    fn signal_list(&mut self) -> &mut Self::SignalL;

    fn buffer_list(&mut self) -> &mut Self::BufferL;
    fn buffer_view_list(&mut self) -> &mut Self::BufferViewL;

    fn sampler_list(&mut self) -> &mut Self::SamplerL;
    fn image_list(&mut self) -> &mut Self::ImageL;
    fn texture_list(&mut self) -> &mut Self::TextureL;

    fn material_list(&mut self) -> &mut Self::MaterialL;
    fn geometry_list(&mut self) -> &mut Self::GeometryL;

    fn table_list(&mut self) -> &mut Self::TableL;
    fn plot_list(&mut self) -> &mut Self::PlotL;

    fn entity_list(&mut self) -> &mut Self::EntityL;

    fn document_update(&mut self, update: ClientDocumentUpdate);
}
