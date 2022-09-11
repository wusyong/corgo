use webrender::{
    api::{units::LayoutSize, *},
    RenderApi, Transaction,
};

use crate::utils::RectBuilder;

pub fn render(
    pipeline_id: PipelineId,
    document_id: DocumentId,
    epoch: Epoch,
    api: &mut RenderApi,
    layout_size: LayoutSize,
) {
    let mut txn = Transaction::new();
    let mut builder = DisplayListBuilder::new(pipeline_id);
    builder.begin();

    {
        //TODO render node
        let content_bounds = units::LayoutRect::from_size(layout_size);
        let root_space_and_clip = SpaceAndClipInfo::root_scroll(pipeline_id);
        let spatial_id = root_space_and_clip.spatial_id;

        builder.push_simple_stacking_context(
            content_bounds.min,
            spatial_id,
            PrimitiveFlags::IS_BACKFACE_VISIBLE,
        );

        builder.pop_stacking_context();
    }

    txn.set_display_list(epoch, None, layout_size, builder.end());
    txn.generate_frame(0, RenderReasons::empty());
    api.send_transaction(document_id, txn);
}
