#include "wrapper.h"

// Static wrappers

bool spa_ptrinside_libspa_rs(const void *p1, size_t s1, const void *p2, size_t s2, size_t *remaining) { return spa_ptrinside(p1, s1, p2, s2, remaining); }
bool spa_ptr_inside_and_aligned_libspa_rs(const void *p1, size_t s1, const void *p2, size_t s2, size_t align, size_t *remaining) { return spa_ptr_inside_and_aligned(p1, s1, p2, s2, align, remaining); }
bool spa_streq_libspa_rs(const char *s1, const char *s2) { return spa_streq(s1, s2); }
bool spa_strneq_libspa_rs(const char *s1, const char *s2, size_t len) { return spa_strneq(s1, s2, len); }
bool spa_strstartswith_libspa_rs(const char *s, const char *prefix) { return spa_strstartswith(s, prefix); }
bool spa_strendswith_libspa_rs(const char *s, const char *suffix) { return spa_strendswith(s, suffix); }
bool spa_atoi32_libspa_rs(const char *str, int32_t *val, int base) { return spa_atoi32(str, val, base); }
bool spa_atou32_libspa_rs(const char *str, uint32_t *val, int base) { return spa_atou32(str, val, base); }
bool spa_atoi64_libspa_rs(const char *str, int64_t *val, int base) { return spa_atoi64(str, val, base); }
bool spa_atou64_libspa_rs(const char *str, uint64_t *val, int base) { return spa_atou64(str, val, base); }
bool spa_atob_libspa_rs(const char *str) { return spa_atob(str); }
int spa_vscnprintf_libspa_rs(char *buffer, size_t size, const char *format, va_list args) { return spa_vscnprintf(buffer, size, format, args); }
float spa_strtof_libspa_rs(const char *str, char **endptr) { return spa_strtof(str, endptr); }
bool spa_atof_libspa_rs(const char *str, float *val) { return spa_atof(str, val); }
double spa_strtod_libspa_rs(const char *str, char **endptr) { return spa_strtod(str, endptr); }
bool spa_atod_libspa_rs(const char *str, double *val) { return spa_atod(str, val); }
char * spa_dtoa_libspa_rs(char *str, size_t size, double val) { return spa_dtoa(str, size, val); }
void spa_strbuf_init_libspa_rs(struct spa_strbuf *buf, char *buffer, size_t maxsize) { spa_strbuf_init(buf, buffer, maxsize); }
bool spa_type_is_a_libspa_rs(const char *type, const char *parent) { return spa_type_is_a(type, parent); }
const char * spa_type_short_name_libspa_rs(const char *name) { return spa_type_short_name(name); }
uint32_t spa_type_from_short_name_libspa_rs(const char *name, const struct spa_type_info *info, uint32_t unknown) { return spa_type_from_short_name(name, info, unknown); }
const char * spa_type_to_name_libspa_rs(uint32_t type, const struct spa_type_info *info, const char *unknown) { return spa_type_to_name(type, info, unknown); }
const char * spa_type_to_short_name_libspa_rs(uint32_t type, const struct spa_type_info *info, const char *unknown) { return spa_type_to_short_name(type, info, unknown); }
void * spa_meta_first_libspa_rs(const struct spa_meta *m) { return spa_meta_first(m); }
void * spa_meta_end_libspa_rs(const struct spa_meta *m) { return spa_meta_end(m); }
bool spa_meta_region_is_valid_libspa_rs(const struct spa_meta_region *m) { return spa_meta_region_is_valid(m); }
bool spa_meta_bitmap_is_valid_libspa_rs(const struct spa_meta_bitmap *m) { return spa_meta_bitmap_is_valid(m); }
bool spa_meta_cursor_is_valid_libspa_rs(const struct spa_meta_cursor *m) { return spa_meta_cursor_is_valid(m); }
struct spa_meta * spa_buffer_find_meta_libspa_rs(const struct spa_buffer *b, uint32_t type) { return spa_buffer_find_meta(b, type); }
void * spa_buffer_find_meta_data_libspa_rs(const struct spa_buffer *b, uint32_t type, size_t size) { return spa_buffer_find_meta_data(b, type, size); }
bool spa_buffer_has_meta_features_libspa_rs(const struct spa_buffer *b, uint32_t type, uint32_t features) { return spa_buffer_has_meta_features(b, type, features); }
int spa_buffer_alloc_fill_info_libspa_rs(struct spa_buffer_alloc_info *info, uint32_t n_metas, struct spa_meta metas [0], uint32_t n_datas, struct spa_data datas [0], uint32_t data_aligns [0]) { return spa_buffer_alloc_fill_info(info, n_metas, metas, n_datas, datas, data_aligns); }
struct spa_buffer * spa_buffer_alloc_layout_libspa_rs(struct spa_buffer_alloc_info *info, void *skel_mem, void *data_mem) { return spa_buffer_alloc_layout(info, skel_mem, data_mem); }
int spa_buffer_alloc_layout_array_libspa_rs(struct spa_buffer_alloc_info *info, uint32_t n_buffers, struct spa_buffer *buffers [0], void *skel_mem, void *data_mem) { return spa_buffer_alloc_layout_array(info, n_buffers, buffers, skel_mem, data_mem); }
struct spa_buffer ** spa_buffer_alloc_array_libspa_rs(uint32_t n_buffers, uint32_t flags, uint32_t n_metas, struct spa_meta metas [0], uint32_t n_datas, struct spa_data datas [0], uint32_t data_aligns [0]) { return spa_buffer_alloc_array(n_buffers, flags, n_metas, metas, n_datas, datas, data_aligns); }
void spa_debugc_error_location_libspa_rs(struct spa_debug_context *c, struct spa_error_location *loc) { spa_debugc_error_location(c, loc); }
int spa_debugc_mem_libspa_rs(struct spa_debug_context *ctx, int indent, const void *data, size_t size) { return spa_debugc_mem(ctx, indent, data, size); }
int spa_debug_mem_libspa_rs(int indent, const void *data, size_t size) { return spa_debug_mem(indent, data, size); }
uint32_t spa_type_audio_format_from_short_name_libspa_rs(const char *name) { return spa_type_audio_format_from_short_name(name); }
const char * spa_type_audio_format_to_short_name_libspa_rs(uint32_t type) { return spa_type_audio_format_to_short_name(type); }
uint32_t spa_type_audio_channel_from_short_name_libspa_rs(const char *name) { return spa_type_audio_channel_from_short_name(name); }
const char * spa_type_audio_channel_to_short_name_libspa_rs(uint32_t type) { return spa_type_audio_channel_to_short_name(type); }
const char * spa_type_audio_channel_make_short_name_libspa_rs(uint32_t type, char *buf, size_t size, const char *unknown) { return spa_type_audio_channel_make_short_name(type, buf, size, unknown); }
int spa_audio_layout_info_parse_name_libspa_rs(struct spa_audio_layout_info *layout, size_t size, const char *name) { return spa_audio_layout_info_parse_name(layout, size, name); }
uint32_t spa_type_audio_iec958_codec_from_short_name_libspa_rs(const char *name) { return spa_type_audio_iec958_codec_from_short_name(name); }
const char * spa_type_audio_iec958_codec_to_short_name_libspa_rs(uint32_t type) { return spa_type_audio_iec958_codec_to_short_name(type); }
uint32_t spa_type_video_format_from_short_name_libspa_rs(const char *name) { return spa_type_video_format_from_short_name(name); }
const char * spa_type_video_format_to_short_name_libspa_rs(uint32_t type) { return spa_type_video_format_to_short_name(type); }
const struct spa_type_info * spa_debug_type_find_libspa_rs(const struct spa_type_info *info, uint32_t type) { return spa_debug_type_find(info, type); }
const char * spa_debug_type_short_name_libspa_rs(const char *name) { return spa_debug_type_short_name(name); }
const char * spa_debug_type_find_name_libspa_rs(const struct spa_type_info *info, uint32_t type) { return spa_debug_type_find_name(info, type); }
const char * spa_debug_type_find_short_name_libspa_rs(const struct spa_type_info *info, uint32_t type) { return spa_debug_type_find_short_name(info, type); }
uint32_t spa_debug_type_find_type_libspa_rs(const struct spa_type_info *info, const char *name) { return spa_debug_type_find_type(info, name); }
const struct spa_type_info * spa_debug_type_find_short_libspa_rs(const struct spa_type_info *info, const char *name) { return spa_debug_type_find_short(info, name); }
uint32_t spa_debug_type_find_type_short_libspa_rs(const struct spa_type_info *info, const char *name) { return spa_debug_type_find_type_short(info, name); }
int spa_debugc_buffer_libspa_rs(struct spa_debug_context *ctx, int indent, const struct spa_buffer *buffer) { return spa_debugc_buffer(ctx, indent, buffer); }
int spa_debug_buffer_libspa_rs(int indent, const struct spa_buffer *buffer) { return spa_debug_buffer(indent, buffer); }
int spa_dict_item_compare_libspa_rs(const void *i1, const void *i2) { return spa_dict_item_compare(i1, i2); }
void spa_dict_qsort_libspa_rs(struct spa_dict *dict) { spa_dict_qsort(dict); }
const struct spa_dict_item * spa_dict_lookup_item_libspa_rs(const struct spa_dict *dict, const char *key) { return spa_dict_lookup_item(dict, key); }
const char * spa_dict_lookup_libspa_rs(const struct spa_dict *dict, const char *key) { return spa_dict_lookup(dict, key); }
int spa_debugc_dict_libspa_rs(struct spa_debug_context *ctx, int indent, const struct spa_dict *dict) { return spa_debugc_dict(ctx, indent, dict); }
int spa_debug_dict_libspa_rs(int indent, const struct spa_dict *dict) { return spa_debug_dict(indent, dict); }
uint32_t spa_pod_type_size_libspa_rs(uint32_t type) { return spa_pod_type_size(type); }
int spa_pod_choice_n_values_libspa_rs(uint32_t choice_type, uint32_t *min, uint32_t *max) { return spa_pod_choice_n_values(choice_type, min, max); }
int spa_pod_body_from_data_libspa_rs(void *data, size_t maxsize, off_t offset, size_t size, struct spa_pod *pod, const void **body) { return spa_pod_body_from_data(data, maxsize, offset, size, pod, body); }
int spa_pod_is_none_libspa_rs(const struct spa_pod *pod) { return spa_pod_is_none(pod); }
int spa_pod_is_bool_libspa_rs(const struct spa_pod *pod) { return spa_pod_is_bool(pod); }
int spa_pod_body_get_bool_libspa_rs(const struct spa_pod *pod, const void *body, bool *value) { return spa_pod_body_get_bool(pod, body, value); }
int spa_pod_is_id_libspa_rs(const struct spa_pod *pod) { return spa_pod_is_id(pod); }
int spa_pod_body_get_id_libspa_rs(const struct spa_pod *pod, const void *body, uint32_t *value) { return spa_pod_body_get_id(pod, body, value); }
int spa_pod_is_int_libspa_rs(const struct spa_pod *pod) { return spa_pod_is_int(pod); }
int spa_pod_body_get_int_libspa_rs(const struct spa_pod *pod, const void *body, int32_t *value) { return spa_pod_body_get_int(pod, body, value); }
int spa_pod_is_long_libspa_rs(const struct spa_pod *pod) { return spa_pod_is_long(pod); }
int spa_pod_body_get_long_libspa_rs(const struct spa_pod *pod, const void *body, int64_t *value) { return spa_pod_body_get_long(pod, body, value); }
int spa_pod_is_float_libspa_rs(const struct spa_pod *pod) { return spa_pod_is_float(pod); }
int spa_pod_body_get_float_libspa_rs(const struct spa_pod *pod, const void *body, float *value) { return spa_pod_body_get_float(pod, body, value); }
int spa_pod_is_double_libspa_rs(const struct spa_pod *pod) { return spa_pod_is_double(pod); }
int spa_pod_body_get_double_libspa_rs(const struct spa_pod *pod, const void *body, double *value) { return spa_pod_body_get_double(pod, body, value); }
int spa_pod_is_string_libspa_rs(const struct spa_pod *pod) { return spa_pod_is_string(pod); }
int spa_pod_body_get_string_libspa_rs(const struct spa_pod *pod, const void *body, const char **value) { return spa_pod_body_get_string(pod, body, value); }
int spa_pod_body_copy_string_libspa_rs(const struct spa_pod *pod, const void *body, char *dest, size_t maxlen) { return spa_pod_body_copy_string(pod, body, dest, maxlen); }
int spa_pod_is_bytes_libspa_rs(const struct spa_pod *pod) { return spa_pod_is_bytes(pod); }
int spa_pod_body_get_bytes_libspa_rs(const struct spa_pod *pod, const void *body, const void **value, uint32_t *len) { return spa_pod_body_get_bytes(pod, body, value, len); }
int spa_pod_is_pointer_libspa_rs(const struct spa_pod *pod) { return spa_pod_is_pointer(pod); }
int spa_pod_body_get_pointer_libspa_rs(const struct spa_pod *pod, const void *body, uint32_t *type, const void **value) { return spa_pod_body_get_pointer(pod, body, type, value); }
int spa_pod_is_fd_libspa_rs(const struct spa_pod *pod) { return spa_pod_is_fd(pod); }
int spa_pod_body_get_fd_libspa_rs(const struct spa_pod *pod, const void *body, int64_t *value) { return spa_pod_body_get_fd(pod, body, value); }
int spa_pod_is_rectangle_libspa_rs(const struct spa_pod *pod) { return spa_pod_is_rectangle(pod); }
int spa_pod_body_get_rectangle_libspa_rs(const struct spa_pod *pod, const void *body, struct spa_rectangle *value) { return spa_pod_body_get_rectangle(pod, body, value); }
int spa_pod_is_fraction_libspa_rs(const struct spa_pod *pod) { return spa_pod_is_fraction(pod); }
int spa_pod_body_get_fraction_libspa_rs(const struct spa_pod *pod, const void *body, struct spa_fraction *value) { return spa_pod_body_get_fraction(pod, body, value); }
int spa_pod_is_bitmap_libspa_rs(const struct spa_pod *pod) { return spa_pod_is_bitmap(pod); }
int spa_pod_body_get_bitmap_libspa_rs(const struct spa_pod *pod, const void *body, const uint8_t **value) { return spa_pod_body_get_bitmap(pod, body, value); }
int spa_pod_is_array_libspa_rs(const struct spa_pod *pod) { return spa_pod_is_array(pod); }
int spa_pod_body_get_array_libspa_rs(const struct spa_pod *pod, const void *body, struct spa_pod_array *arr, const void **arr_body) { return spa_pod_body_get_array(pod, body, arr, arr_body); }
const void * spa_pod_array_body_get_values_libspa_rs(const struct spa_pod_array *arr, const void *body, uint32_t *n_values, uint32_t *val_size, uint32_t *val_type) { return spa_pod_array_body_get_values(arr, body, n_values, val_size, val_type); }
const void * spa_pod_body_get_array_values_libspa_rs(const struct spa_pod *pod, const void *body, uint32_t *n_values, uint32_t *val_size, uint32_t *val_type) { return spa_pod_body_get_array_values(pod, body, n_values, val_size, val_type); }
int spa_pod_is_choice_libspa_rs(const struct spa_pod *pod) { return spa_pod_is_choice(pod); }
int spa_pod_body_get_choice_libspa_rs(const struct spa_pod *pod, const void *body, struct spa_pod_choice *choice, const void **choice_body) { return spa_pod_body_get_choice(pod, body, choice, choice_body); }
const void * spa_pod_choice_body_get_values_libspa_rs(const struct spa_pod_choice *pod, const void *body, uint32_t *n_values, uint32_t *choice, uint32_t *val_size, uint32_t *val_type) { return spa_pod_choice_body_get_values(pod, body, n_values, choice, val_size, val_type); }
int spa_pod_is_struct_libspa_rs(const struct spa_pod *pod) { return spa_pod_is_struct(pod); }
int spa_pod_is_object_libspa_rs(const struct spa_pod *pod) { return spa_pod_is_object(pod); }
int spa_pod_body_get_object_libspa_rs(const struct spa_pod *pod, const void *body, struct spa_pod_object *object, const void **object_body) { return spa_pod_body_get_object(pod, body, object, object_body); }
int spa_pod_is_sequence_libspa_rs(const struct spa_pod *pod) { return spa_pod_is_sequence(pod); }
int spa_pod_body_get_sequence_libspa_rs(const struct spa_pod *pod, const void *body, struct spa_pod_sequence *seq, const void **seq_body) { return spa_pod_body_get_sequence(pod, body, seq, seq_body); }
bool spa_pod_is_inside_libspa_rs(const void *pod, uint32_t size, const void *iter) { return spa_pod_is_inside(pod, size, iter); }
void * spa_pod_next_libspa_rs(const void *iter) { return spa_pod_next(iter); }
struct spa_pod_prop * spa_pod_prop_first_libspa_rs(const struct spa_pod_object_body *body) { return spa_pod_prop_first(body); }
bool spa_pod_prop_is_inside_libspa_rs(const struct spa_pod_object_body *body, uint32_t size, const struct spa_pod_prop *iter) { return spa_pod_prop_is_inside(body, size, iter); }
struct spa_pod_prop * spa_pod_prop_next_libspa_rs(const struct spa_pod_prop *iter) { return spa_pod_prop_next(iter); }
struct spa_pod_control * spa_pod_control_first_libspa_rs(const struct spa_pod_sequence_body *body) { return spa_pod_control_first(body); }
bool spa_pod_control_is_inside_libspa_rs(const struct spa_pod_sequence_body *body, uint32_t size, const struct spa_pod_control *iter) { return spa_pod_control_is_inside(body, size, iter); }
struct spa_pod_control * spa_pod_control_next_libspa_rs(const struct spa_pod_control *iter) { return spa_pod_control_next(iter); }
void * spa_pod_from_data_libspa_rs(void *data, size_t maxsize, off_t offset, size_t size) { return spa_pod_from_data(data, maxsize, offset, size); }
int spa_pod_get_bool_libspa_rs(const struct spa_pod *pod, bool *value) { return spa_pod_get_bool(pod, value); }
int spa_pod_get_id_libspa_rs(const struct spa_pod *pod, uint32_t *value) { return spa_pod_get_id(pod, value); }
int spa_pod_get_int_libspa_rs(const struct spa_pod *pod, int32_t *value) { return spa_pod_get_int(pod, value); }
int spa_pod_get_long_libspa_rs(const struct spa_pod *pod, int64_t *value) { return spa_pod_get_long(pod, value); }
int spa_pod_get_float_libspa_rs(const struct spa_pod *pod, float *value) { return spa_pod_get_float(pod, value); }
int spa_pod_get_double_libspa_rs(const struct spa_pod *pod, double *value) { return spa_pod_get_double(pod, value); }
int spa_pod_get_string_libspa_rs(const struct spa_pod *pod, const char **value) { return spa_pod_get_string(pod, value); }
int spa_pod_copy_string_libspa_rs(const struct spa_pod *pod, size_t maxlen, char *dest) { return spa_pod_copy_string(pod, maxlen, dest); }
int spa_pod_get_bytes_libspa_rs(const struct spa_pod *pod, const void **value, uint32_t *len) { return spa_pod_get_bytes(pod, value, len); }
int spa_pod_get_pointer_libspa_rs(const struct spa_pod *pod, uint32_t *type, const void **value) { return spa_pod_get_pointer(pod, type, value); }
int spa_pod_get_fd_libspa_rs(const struct spa_pod *pod, int64_t *value) { return spa_pod_get_fd(pod, value); }
int spa_pod_get_rectangle_libspa_rs(const struct spa_pod *pod, struct spa_rectangle *value) { return spa_pod_get_rectangle(pod, value); }
int spa_pod_get_fraction_libspa_rs(const struct spa_pod *pod, struct spa_fraction *value) { return spa_pod_get_fraction(pod, value); }
void * spa_pod_get_array_full_libspa_rs(const struct spa_pod *pod, uint32_t *n_values, uint32_t *val_size, uint32_t *val_type) { return spa_pod_get_array_full(pod, n_values, val_size, val_type); }
void * spa_pod_get_array_libspa_rs(const struct spa_pod *pod, uint32_t *n_values) { return spa_pod_get_array(pod, n_values); }
uint32_t spa_pod_copy_array_full_libspa_rs(const struct spa_pod *pod, uint32_t type, uint32_t size, void *values, uint32_t max_values) { return spa_pod_copy_array_full(pod, type, size, values, max_values); }
struct spa_pod * spa_pod_get_values_libspa_rs(const struct spa_pod *pod, uint32_t *n_vals, uint32_t *choice) { return spa_pod_get_values(pod, n_vals, choice); }
bool spa_pod_is_object_type_libspa_rs(const struct spa_pod *pod, uint32_t type) { return spa_pod_is_object_type(pod, type); }
bool spa_pod_is_object_id_libspa_rs(const struct spa_pod *pod, uint32_t id) { return spa_pod_is_object_id(pod, id); }
const struct spa_pod_prop * spa_pod_object_find_prop_libspa_rs(const struct spa_pod_object *pod, const struct spa_pod_prop *start, uint32_t key) { return spa_pod_object_find_prop(pod, start, key); }
const struct spa_pod_prop * spa_pod_find_prop_libspa_rs(const struct spa_pod *pod, const struct spa_pod_prop *start, uint32_t key) { return spa_pod_find_prop(pod, start, key); }
int spa_pod_object_has_props_libspa_rs(const struct spa_pod_object *pod) { return spa_pod_object_has_props(pod); }
int spa_pod_object_fixate_libspa_rs(struct spa_pod_object *pod) { return spa_pod_object_fixate(pod); }
int spa_pod_object_is_fixated_libspa_rs(const struct spa_pod_object *pod) { return spa_pod_object_is_fixated(pod); }
int spa_pod_fixate_libspa_rs(struct spa_pod *pod) { return spa_pod_fixate(pod); }
int spa_pod_is_fixated_libspa_rs(const struct spa_pod *pod) { return spa_pod_is_fixated(pod); }
void spa_pod_parser_init_libspa_rs(struct spa_pod_parser *parser, const void *data, uint32_t size) { spa_pod_parser_init(parser, data, size); }
void spa_pod_parser_pod_libspa_rs(struct spa_pod_parser *parser, const struct spa_pod *pod) { spa_pod_parser_pod(parser, pod); }
void spa_pod_parser_init_pod_body_libspa_rs(struct spa_pod_parser *parser, const struct spa_pod *pod, const void *body) { spa_pod_parser_init_pod_body(parser, pod, body); }
void spa_pod_parser_init_from_data_libspa_rs(struct spa_pod_parser *parser, const void *data, uint32_t maxsize, uint32_t offset, uint32_t size) { spa_pod_parser_init_from_data(parser, data, maxsize, offset, size); }
void spa_pod_parser_get_state_libspa_rs(struct spa_pod_parser *parser, struct spa_pod_parser_state *state) { spa_pod_parser_get_state(parser, state); }
void spa_pod_parser_reset_libspa_rs(struct spa_pod_parser *parser, struct spa_pod_parser_state *state) { spa_pod_parser_reset(parser, state); }
int spa_pod_parser_read_header_libspa_rs(struct spa_pod_parser *parser, uint32_t offset, uint32_t size, void *header, uint32_t header_size, uint32_t pod_offset, const void **body) { return spa_pod_parser_read_header(parser, offset, size, header, header_size, pod_offset, body); }
struct spa_pod * spa_pod_parser_deref_libspa_rs(struct spa_pod_parser *parser, uint32_t offset, uint32_t size) { return spa_pod_parser_deref(parser, offset, size); }
struct spa_pod * spa_pod_parser_frame_libspa_rs(struct spa_pod_parser *parser, struct spa_pod_frame *frame) { return spa_pod_parser_frame(parser, frame); }
void spa_pod_parser_push_libspa_rs(struct spa_pod_parser *parser, struct spa_pod_frame *frame, const struct spa_pod *pod, uint32_t offset) { spa_pod_parser_push(parser, frame, pod, offset); }
int spa_pod_parser_get_header_libspa_rs(struct spa_pod_parser *parser, void *header, uint32_t header_size, uint32_t pod_offset, const void **body) { return spa_pod_parser_get_header(parser, header, header_size, pod_offset, body); }
int spa_pod_parser_current_body_libspa_rs(struct spa_pod_parser *parser, struct spa_pod *pod, const void **body) { return spa_pod_parser_current_body(parser, pod, body); }
struct spa_pod * spa_pod_parser_current_libspa_rs(struct spa_pod_parser *parser) { return spa_pod_parser_current(parser); }
void spa_pod_parser_advance_libspa_rs(struct spa_pod_parser *parser, const struct spa_pod *pod) { spa_pod_parser_advance(parser, pod); }
int spa_pod_parser_next_body_libspa_rs(struct spa_pod_parser *parser, struct spa_pod *pod, const void **body) { return spa_pod_parser_next_body(parser, pod, body); }
struct spa_pod * spa_pod_parser_next_libspa_rs(struct spa_pod_parser *parser) { return spa_pod_parser_next(parser); }
void spa_pod_parser_restart_libspa_rs(struct spa_pod_parser *parser, struct spa_pod_frame *frame) { spa_pod_parser_restart(parser, frame); }
void spa_pod_parser_unpush_libspa_rs(struct spa_pod_parser *parser, struct spa_pod_frame *frame) { spa_pod_parser_unpush(parser, frame); }
int spa_pod_parser_pop_libspa_rs(struct spa_pod_parser *parser, struct spa_pod_frame *frame) { return spa_pod_parser_pop(parser, frame); }
int spa_pod_parser_get_bool_libspa_rs(struct spa_pod_parser *parser, bool *value) { return spa_pod_parser_get_bool(parser, value); }
int spa_pod_parser_get_id_libspa_rs(struct spa_pod_parser *parser, uint32_t *value) { return spa_pod_parser_get_id(parser, value); }
int spa_pod_parser_get_int_libspa_rs(struct spa_pod_parser *parser, int32_t *value) { return spa_pod_parser_get_int(parser, value); }
int spa_pod_parser_get_long_libspa_rs(struct spa_pod_parser *parser, int64_t *value) { return spa_pod_parser_get_long(parser, value); }
int spa_pod_parser_get_float_libspa_rs(struct spa_pod_parser *parser, float *value) { return spa_pod_parser_get_float(parser, value); }
int spa_pod_parser_get_double_libspa_rs(struct spa_pod_parser *parser, double *value) { return spa_pod_parser_get_double(parser, value); }
int spa_pod_parser_get_string_libspa_rs(struct spa_pod_parser *parser, const char **value) { return spa_pod_parser_get_string(parser, value); }
int spa_pod_parser_get_bytes_libspa_rs(struct spa_pod_parser *parser, const void **value, uint32_t *len) { return spa_pod_parser_get_bytes(parser, value, len); }
int spa_pod_parser_get_pointer_libspa_rs(struct spa_pod_parser *parser, uint32_t *type, const void **value) { return spa_pod_parser_get_pointer(parser, type, value); }
int spa_pod_parser_get_fd_libspa_rs(struct spa_pod_parser *parser, int64_t *value) { return spa_pod_parser_get_fd(parser, value); }
int spa_pod_parser_get_rectangle_libspa_rs(struct spa_pod_parser *parser, struct spa_rectangle *value) { return spa_pod_parser_get_rectangle(parser, value); }
int spa_pod_parser_get_fraction_libspa_rs(struct spa_pod_parser *parser, struct spa_fraction *value) { return spa_pod_parser_get_fraction(parser, value); }
int spa_pod_parser_get_pod_body_libspa_rs(struct spa_pod_parser *parser, struct spa_pod *value, const void **body) { return spa_pod_parser_get_pod_body(parser, value, body); }
int spa_pod_parser_get_pod_libspa_rs(struct spa_pod_parser *parser, struct spa_pod **value) { return spa_pod_parser_get_pod(parser, value); }
int spa_pod_parser_init_struct_body_libspa_rs(struct spa_pod_parser *parser, struct spa_pod_frame *frame, const struct spa_pod *pod, const void *body) { return spa_pod_parser_init_struct_body(parser, frame, pod, body); }
int spa_pod_parser_push_struct_body_libspa_rs(struct spa_pod_parser *parser, struct spa_pod_frame *frame, struct spa_pod *str, const void **str_body) { return spa_pod_parser_push_struct_body(parser, frame, str, str_body); }
int spa_pod_parser_push_struct_libspa_rs(struct spa_pod_parser *parser, struct spa_pod_frame *frame) { return spa_pod_parser_push_struct(parser, frame); }
int spa_pod_parser_init_object_body_libspa_rs(struct spa_pod_parser *parser, struct spa_pod_frame *frame, const struct spa_pod *pod, const void *body, struct spa_pod_object *object, const void **object_body) { return spa_pod_parser_init_object_body(parser, frame, pod, body, object, object_body); }
int spa_pod_parser_push_object_body_libspa_rs(struct spa_pod_parser *parser, struct spa_pod_frame *frame, struct spa_pod_object *object, const void **object_body) { return spa_pod_parser_push_object_body(parser, frame, object, object_body); }
int spa_pod_parser_push_object_libspa_rs(struct spa_pod_parser *parser, struct spa_pod_frame *frame, uint32_t type, uint32_t *id) { return spa_pod_parser_push_object(parser, frame, type, id); }
int spa_pod_parser_get_prop_body_libspa_rs(struct spa_pod_parser *parser, struct spa_pod_prop *prop, const void **body) { return spa_pod_parser_get_prop_body(parser, prop, body); }
int spa_pod_parser_push_sequence_body_libspa_rs(struct spa_pod_parser *parser, struct spa_pod_frame *frame, struct spa_pod_sequence *seq, const void **seq_body) { return spa_pod_parser_push_sequence_body(parser, frame, seq, seq_body); }
int spa_pod_parser_get_control_body_libspa_rs(struct spa_pod_parser *parser, struct spa_pod_control *control, const void **body) { return spa_pod_parser_get_control_body(parser, control, body); }
int spa_pod_parser_object_find_prop_libspa_rs(struct spa_pod_parser *parser, uint32_t key, struct spa_pod_prop *prop, const void **body) { return spa_pod_parser_object_find_prop(parser, key, prop, body); }
bool spa_pod_parser_body_can_collect_libspa_rs(const struct spa_pod *pod, const void *body, char type) { return spa_pod_parser_body_can_collect(pod, body, type); }
bool spa_pod_parser_can_collect_libspa_rs(const struct spa_pod *pod, char type) { return spa_pod_parser_can_collect(pod, type); }
int spa_pod_parser_getv_libspa_rs(struct spa_pod_parser *parser, va_list args) { return spa_pod_parser_getv(parser, args); }
int spa_format_parse_libspa_rs(const struct spa_pod *format, uint32_t *media_type, uint32_t *media_subtype) { return spa_format_parse(format, media_type, media_subtype); }
int spa_debug_strbuf_format_value_libspa_rs(struct spa_strbuf *buffer, const struct spa_type_info *info, uint32_t type, void *body, uint32_t size) { return spa_debug_strbuf_format_value(buffer, info, type, body, size); }
int spa_debug_format_value_libspa_rs(const struct spa_type_info *info, uint32_t type, void *body, uint32_t size) { return spa_debug_format_value(info, type, body, size); }
int spa_debugc_format_libspa_rs(struct spa_debug_context *ctx, int indent, const struct spa_type_info *info, const struct spa_pod *format) { return spa_debugc_format(ctx, indent, info, format); }
int spa_debug_format_libspa_rs(int indent, const struct spa_type_info *info, const struct spa_pod *format) { return spa_debug_format(indent, info, format); }
void spa_list_init_libspa_rs(struct spa_list *list) { spa_list_init(list); }
int spa_list_is_initialized_libspa_rs(struct spa_list *list) { return spa_list_is_initialized(list); }
void spa_list_insert_libspa_rs(struct spa_list *list, struct spa_list *elem) { spa_list_insert(list, elem); }
void spa_list_insert_list_libspa_rs(struct spa_list *list, struct spa_list *other) { spa_list_insert_list(list, other); }
void spa_list_remove_libspa_rs(struct spa_list *elem) { spa_list_remove(elem); }
void spa_hook_list_init_libspa_rs(struct spa_hook_list *list) { spa_hook_list_init(list); }
bool spa_hook_list_is_empty_libspa_rs(struct spa_hook_list *list) { return spa_hook_list_is_empty(list); }
void spa_hook_list_append_libspa_rs(struct spa_hook_list *list, struct spa_hook *hook, const void *funcs, void *data) { spa_hook_list_append(list, hook, funcs, data); }
void spa_hook_list_prepend_libspa_rs(struct spa_hook_list *list, struct spa_hook *hook, const void *funcs, void *data) { spa_hook_list_prepend(list, hook, funcs, data); }
void spa_hook_remove_libspa_rs(struct spa_hook *hook) { spa_hook_remove(hook); }
void spa_hook_list_clean_libspa_rs(struct spa_hook_list *list) { spa_hook_list_clean(list); }
void spa_hook_list_isolate_libspa_rs(struct spa_hook_list *list, struct spa_hook_list *save, struct spa_hook *hook, const void *funcs, void *data) { spa_hook_list_isolate(list, save, hook, funcs, data); }
void spa_hook_list_join_libspa_rs(struct spa_hook_list *list, struct spa_hook_list *save) { spa_hook_list_join(list, save); }
int spa_node_add_listener_libspa_rs(struct spa_node *object, struct spa_hook *listener, const struct spa_node_events *events, void *data) { return spa_node_add_listener(object, listener, events, data); }
int spa_node_set_callbacks_libspa_rs(struct spa_node *object, const struct spa_node_callbacks *callbacks, void *data) { return spa_node_set_callbacks(object, callbacks, data); }
int spa_node_sync_libspa_rs(struct spa_node *object, int seq) { return spa_node_sync(object, seq); }
int spa_node_enum_params_libspa_rs(struct spa_node *object, int seq, uint32_t id, uint32_t start, uint32_t max, const struct spa_pod *filter) { return spa_node_enum_params(object, seq, id, start, max, filter); }
int spa_node_set_param_libspa_rs(struct spa_node *object, uint32_t id, uint32_t flags, const struct spa_pod *param) { return spa_node_set_param(object, id, flags, param); }
int spa_node_set_io_libspa_rs(struct spa_node *object, uint32_t id, void *data, size_t size) { return spa_node_set_io(object, id, data, size); }
int spa_node_send_command_libspa_rs(struct spa_node *object, const struct spa_command *command) { return spa_node_send_command(object, command); }
int spa_node_add_port_libspa_rs(struct spa_node *object, enum spa_direction direction, uint32_t port_id, const struct spa_dict *props) { return spa_node_add_port(object, direction, port_id, props); }
int spa_node_remove_port_libspa_rs(struct spa_node *object, enum spa_direction direction, uint32_t port_id) { return spa_node_remove_port(object, direction, port_id); }
int spa_node_port_enum_params_libspa_rs(struct spa_node *object, int seq, enum spa_direction direction, uint32_t port_id, uint32_t id, uint32_t start, uint32_t max, const struct spa_pod *filter) { return spa_node_port_enum_params(object, seq, direction, port_id, id, start, max, filter); }
int spa_node_port_set_param_libspa_rs(struct spa_node *object, enum spa_direction direction, uint32_t port_id, uint32_t id, uint32_t flags, const struct spa_pod *param) { return spa_node_port_set_param(object, direction, port_id, id, flags, param); }
int spa_node_port_use_buffers_libspa_rs(struct spa_node *object, enum spa_direction direction, uint32_t port_id, uint32_t flags, struct spa_buffer **buffers, uint32_t n_buffers) { return spa_node_port_use_buffers(object, direction, port_id, flags, buffers, n_buffers); }
int spa_node_port_set_io_libspa_rs(struct spa_node *object, enum spa_direction direction, uint32_t port_id, uint32_t id, void *data, size_t size) { return spa_node_port_set_io(object, direction, port_id, id, data, size); }
int spa_node_port_reuse_buffer_libspa_rs(struct spa_node *object, uint32_t port_id, uint32_t buffer_id) { return spa_node_port_reuse_buffer(object, port_id, buffer_id); }
int spa_node_port_reuse_buffer_fast_libspa_rs(struct spa_node *object, uint32_t port_id, uint32_t buffer_id) { return spa_node_port_reuse_buffer_fast(object, port_id, buffer_id); }
int spa_node_process_libspa_rs(struct spa_node *object) { return spa_node_process(object); }
int spa_node_process_fast_libspa_rs(struct spa_node *object) { return spa_node_process_fast(object); }
int spa_debugc_port_info_libspa_rs(struct spa_debug_context *ctx, int indent, const struct spa_port_info *info) { return spa_debugc_port_info(ctx, indent, info); }
int spa_debug_port_info_libspa_rs(int indent, const struct spa_port_info *info) { return spa_debug_port_info(indent, info); }
int spa_debugc_pod_value_libspa_rs(struct spa_debug_context *ctx, int indent, const struct spa_type_info *info, uint32_t type, void *body, uint32_t size) { return spa_debugc_pod_value(ctx, indent, info, type, body, size); }
int spa_debugc_pod_libspa_rs(struct spa_debug_context *ctx, int indent, const struct spa_type_info *info, const struct spa_pod *pod) { return spa_debugc_pod(ctx, indent, info, pod); }
int spa_debug_pod_value_libspa_rs(int indent, const struct spa_type_info *info, uint32_t type, void *body, uint32_t size) { return spa_debug_pod_value(indent, info, type, body, size); }
int spa_debug_pod_libspa_rs(int indent, const struct spa_type_info *info, const struct spa_pod *pod) { return spa_debug_pod(indent, info, pod); }
void spa_graph_state_reset_libspa_rs(struct spa_graph_state *state) { spa_graph_state_reset(state); }
int spa_graph_link_trigger_libspa_rs(struct spa_graph_link *link) { return spa_graph_link_trigger(link); }
int spa_graph_node_trigger_libspa_rs(struct spa_graph_node *node) { return spa_graph_node_trigger(node); }
int spa_graph_run_libspa_rs(struct spa_graph *graph) { return spa_graph_run(graph); }
int spa_graph_finish_libspa_rs(struct spa_graph *graph) { return spa_graph_finish(graph); }
int spa_graph_link_signal_node_libspa_rs(void *data) { return spa_graph_link_signal_node(data); }
int spa_graph_link_signal_graph_libspa_rs(void *data) { return spa_graph_link_signal_graph(data); }
void spa_graph_init_libspa_rs(struct spa_graph *graph, struct spa_graph_state *state) { spa_graph_init(graph, state); }
void spa_graph_link_add_libspa_rs(struct spa_graph_node *out, struct spa_graph_state *state, struct spa_graph_link *link) { spa_graph_link_add(out, state, link); }
void spa_graph_link_remove_libspa_rs(struct spa_graph_link *link) { spa_graph_link_remove(link); }
void spa_graph_node_init_libspa_rs(struct spa_graph_node *node, struct spa_graph_state *state) { spa_graph_node_init(node, state); }
int spa_graph_node_impl_sub_process_libspa_rs(void *data, struct spa_graph_node *node) { return spa_graph_node_impl_sub_process(data, node); }
void spa_graph_node_set_subgraph_libspa_rs(struct spa_graph_node *node, struct spa_graph *subgraph) { spa_graph_node_set_subgraph(node, subgraph); }
void spa_graph_node_set_callbacks_libspa_rs(struct spa_graph_node *node, const struct spa_graph_node_callbacks *callbacks, void *data) { spa_graph_node_set_callbacks(node, callbacks, data); }
void spa_graph_node_add_libspa_rs(struct spa_graph *graph, struct spa_graph_node *node) { spa_graph_node_add(graph, node); }
void spa_graph_node_remove_libspa_rs(struct spa_graph_node *node) { spa_graph_node_remove(node); }
void spa_graph_port_init_libspa_rs(struct spa_graph_port *port, enum spa_direction direction, uint32_t port_id, uint32_t flags) { spa_graph_port_init(port, direction, port_id, flags); }
void spa_graph_port_add_libspa_rs(struct spa_graph_node *node, struct spa_graph_port *port) { spa_graph_port_add(node, port); }
void spa_graph_port_remove_libspa_rs(struct spa_graph_port *port) { spa_graph_port_remove(port); }
void spa_graph_port_link_libspa_rs(struct spa_graph_port *out, struct spa_graph_port *in) { spa_graph_port_link(out, in); }
void spa_graph_port_unlink_libspa_rs(struct spa_graph_port *port) { spa_graph_port_unlink(port); }
int spa_graph_node_impl_process_libspa_rs(void *data, struct spa_graph_node *node) { return spa_graph_node_impl_process(data, node); }
int spa_graph_node_impl_reuse_buffer_libspa_rs(void *data, struct spa_graph_node *node, uint32_t port_id, uint32_t buffer_id) { return spa_graph_node_impl_reuse_buffer(data, node, port_id, buffer_id); }
int spa_device_add_listener_libspa_rs(struct spa_device *object, struct spa_hook *listener, const struct spa_device_events *events, void *data) { return spa_device_add_listener(object, listener, events, data); }
int spa_device_sync_libspa_rs(struct spa_device *object, int seq) { return spa_device_sync(object, seq); }
int spa_device_enum_params_libspa_rs(struct spa_device *object, int seq, uint32_t id, uint32_t index, uint32_t max, const struct spa_pod *filter) { return spa_device_enum_params(object, seq, id, index, max, filter); }
int spa_device_set_param_libspa_rs(struct spa_device *object, uint32_t id, uint32_t flags, const struct spa_pod *param) { return spa_device_set_param(object, id, flags, param); }
void spa_pod_builder_get_state_libspa_rs(struct spa_pod_builder *builder, struct spa_pod_builder_state *state) { spa_pod_builder_get_state(builder, state); }
bool spa_pod_builder_corrupted_libspa_rs(const struct spa_pod_builder *builder) { return spa_pod_builder_corrupted(builder); }
void spa_pod_builder_set_callbacks_libspa_rs(struct spa_pod_builder *builder, const struct spa_pod_builder_callbacks *callbacks, void *data) { spa_pod_builder_set_callbacks(builder, callbacks, data); }
void spa_pod_builder_reset_libspa_rs(struct spa_pod_builder *builder, struct spa_pod_builder_state *state) { spa_pod_builder_reset(builder, state); }
void spa_pod_builder_init_libspa_rs(struct spa_pod_builder *builder, void *data, uint32_t size) { spa_pod_builder_init(builder, data, size); }
struct spa_pod * spa_pod_builder_deref_fallback_libspa_rs(struct spa_pod_builder *builder, uint32_t offset, struct spa_pod *fallback) { return spa_pod_builder_deref_fallback(builder, offset, fallback); }
struct spa_pod * spa_pod_builder_deref_libspa_rs(struct spa_pod_builder *builder, uint32_t offset) { return spa_pod_builder_deref(builder, offset); }
struct spa_pod * spa_pod_builder_frame_libspa_rs(struct spa_pod_builder *builder, struct spa_pod_frame *frame) { return spa_pod_builder_frame(builder, frame); }
void spa_pod_builder_push_libspa_rs(struct spa_pod_builder *builder, struct spa_pod_frame *frame, const struct spa_pod *pod, uint32_t offset) { spa_pod_builder_push(builder, frame, pod, offset); }
int spa_pod_builder_raw_libspa_rs(struct spa_pod_builder *builder, const void *data, uint32_t size) { return spa_pod_builder_raw(builder, data, size); }
void spa_pod_builder_remove_libspa_rs(struct spa_pod_builder *builder, uint32_t size) { spa_pod_builder_remove(builder, size); }
int spa_pod_builder_pad_libspa_rs(struct spa_pod_builder *builder, uint32_t size) { return spa_pod_builder_pad(builder, size); }
int spa_pod_builder_raw_padded_libspa_rs(struct spa_pod_builder *builder, const void *data, uint32_t size) { return spa_pod_builder_raw_padded(builder, data, size); }
void * spa_pod_builder_pop_libspa_rs(struct spa_pod_builder *builder, struct spa_pod_frame *frame) { return spa_pod_builder_pop(builder, frame); }
int spa_pod_builder_primitive_body_libspa_rs(struct spa_pod_builder *builder, const struct spa_pod *p, const void *body, uint32_t body_size, const char *suffix, uint32_t suffix_size) { return spa_pod_builder_primitive_body(builder, p, body, body_size, suffix, suffix_size); }
int spa_pod_builder_primitive_libspa_rs(struct spa_pod_builder *builder, const struct spa_pod *p) { return spa_pod_builder_primitive(builder, p); }
int spa_pod_builder_none_libspa_rs(struct spa_pod_builder *builder) { return spa_pod_builder_none(builder); }
int spa_pod_builder_child_libspa_rs(struct spa_pod_builder *builder, uint32_t size, uint32_t type) { return spa_pod_builder_child(builder, size, type); }
int spa_pod_builder_bool_libspa_rs(struct spa_pod_builder *builder, bool val) { return spa_pod_builder_bool(builder, val); }
int spa_pod_builder_id_libspa_rs(struct spa_pod_builder *builder, uint32_t val) { return spa_pod_builder_id(builder, val); }
int spa_pod_builder_int_libspa_rs(struct spa_pod_builder *builder, int32_t val) { return spa_pod_builder_int(builder, val); }
int spa_pod_builder_long_libspa_rs(struct spa_pod_builder *builder, int64_t val) { return spa_pod_builder_long(builder, val); }
int spa_pod_builder_float_libspa_rs(struct spa_pod_builder *builder, float val) { return spa_pod_builder_float(builder, val); }
int spa_pod_builder_double_libspa_rs(struct spa_pod_builder *builder, double val) { return spa_pod_builder_double(builder, val); }
int spa_pod_builder_write_string_libspa_rs(struct spa_pod_builder *builder, const char *str, uint32_t len) { return spa_pod_builder_write_string(builder, str, len); }
int spa_pod_builder_string_len_libspa_rs(struct spa_pod_builder *builder, const char *str, uint32_t len) { return spa_pod_builder_string_len(builder, str, len); }
int spa_pod_builder_string_libspa_rs(struct spa_pod_builder *builder, const char *str) { return spa_pod_builder_string(builder, str); }
int spa_pod_builder_bytes_libspa_rs(struct spa_pod_builder *builder, const void *bytes, uint32_t len) { return spa_pod_builder_bytes(builder, bytes, len); }
void * spa_pod_builder_reserve_bytes_libspa_rs(struct spa_pod_builder *builder, uint32_t len) { return spa_pod_builder_reserve_bytes(builder, len); }
int spa_pod_builder_pointer_libspa_rs(struct spa_pod_builder *builder, uint32_t type, const void *val) { return spa_pod_builder_pointer(builder, type, val); }
int spa_pod_builder_fd_libspa_rs(struct spa_pod_builder *builder, int64_t fd) { return spa_pod_builder_fd(builder, fd); }
int spa_pod_builder_rectangle_libspa_rs(struct spa_pod_builder *builder, uint32_t width, uint32_t height) { return spa_pod_builder_rectangle(builder, width, height); }
int spa_pod_builder_fraction_libspa_rs(struct spa_pod_builder *builder, uint32_t num, uint32_t denom) { return spa_pod_builder_fraction(builder, num, denom); }
int spa_pod_builder_push_array_libspa_rs(struct spa_pod_builder *builder, struct spa_pod_frame *frame) { return spa_pod_builder_push_array(builder, frame); }
int spa_pod_builder_array_libspa_rs(struct spa_pod_builder *builder, uint32_t child_size, uint32_t child_type, uint32_t n_elems, const void *elems) { return spa_pod_builder_array(builder, child_size, child_type, n_elems, elems); }
int spa_pod_builder_push_choice_libspa_rs(struct spa_pod_builder *builder, struct spa_pod_frame *frame, uint32_t type, uint32_t flags) { return spa_pod_builder_push_choice(builder, frame, type, flags); }
int spa_pod_builder_push_struct_libspa_rs(struct spa_pod_builder *builder, struct spa_pod_frame *frame) { return spa_pod_builder_push_struct(builder, frame); }
int spa_pod_builder_push_object_libspa_rs(struct spa_pod_builder *builder, struct spa_pod_frame *frame, uint32_t type, uint32_t id) { return spa_pod_builder_push_object(builder, frame, type, id); }
int spa_pod_builder_prop_libspa_rs(struct spa_pod_builder *builder, uint32_t key, uint32_t flags) { return spa_pod_builder_prop(builder, key, flags); }
int spa_pod_builder_push_sequence_libspa_rs(struct spa_pod_builder *builder, struct spa_pod_frame *frame, uint32_t unit) { return spa_pod_builder_push_sequence(builder, frame, unit); }
int spa_pod_builder_control_libspa_rs(struct spa_pod_builder *builder, uint32_t offset, uint32_t type) { return spa_pod_builder_control(builder, offset, type); }
uint32_t spa_choice_from_id_flags_libspa_rs(char id, uint32_t *flags) { return spa_choice_from_id_flags(id, flags); }
uint32_t spa_choice_from_id_libspa_rs(char id) { return spa_choice_from_id(id); }
int spa_pod_builder_addv_libspa_rs(struct spa_pod_builder *builder, va_list args) { return spa_pod_builder_addv(builder, args); }
struct spa_pod * spa_pod_copy_libspa_rs(const struct spa_pod *pod) { return spa_pod_copy(pod); }
void spa_result_func_device_params_libspa_rs(void *data, int seq, int res, uint32_t type, const void *result) { spa_result_func_device_params(data, seq, res, type, result); }
int spa_device_enum_params_sync_libspa_rs(struct spa_device *device, uint32_t id, uint32_t *index, const struct spa_pod *filter, struct spa_pod **param, struct spa_pod_builder *builder) { return spa_device_enum_params_sync(device, id, index, filter, param, builder); }
void spa_result_func_node_params_libspa_rs(void *data, int seq, int res, uint32_t type, const void *result) { spa_result_func_node_params(data, seq, res, type, result); }
int spa_node_enum_params_sync_libspa_rs(struct spa_node *node, uint32_t id, uint32_t *index, const struct spa_pod *filter, struct spa_pod **param, struct spa_pod_builder *builder) { return spa_node_enum_params_sync(node, id, index, filter, param, builder); }
int spa_node_port_enum_params_sync_libspa_rs(struct spa_node *node, enum spa_direction direction, uint32_t port_id, uint32_t id, uint32_t *index, const struct spa_pod *filter, struct spa_pod **param, struct spa_pod_builder *builder) { return spa_node_port_enum_params_sync(node, direction, port_id, id, index, filter, param, builder); }
int spa_latency_info_compare_libspa_rs(const struct spa_latency_info *a, const struct spa_latency_info *b) { return spa_latency_info_compare(a, b); }
void spa_latency_info_combine_start_libspa_rs(struct spa_latency_info *info, enum spa_direction direction) { spa_latency_info_combine_start(info, direction); }
void spa_latency_info_combine_finish_libspa_rs(struct spa_latency_info *info) { spa_latency_info_combine_finish(info); }
int spa_latency_info_combine_libspa_rs(struct spa_latency_info *info, const struct spa_latency_info *other) { return spa_latency_info_combine(info, other); }
int spa_latency_parse_libspa_rs(const struct spa_pod *latency, struct spa_latency_info *info) { return spa_latency_parse(latency, info); }
struct spa_pod * spa_latency_build_libspa_rs(struct spa_pod_builder *builder, uint32_t id, const struct spa_latency_info *info) { return spa_latency_build(builder, id, info); }
int spa_process_latency_parse_libspa_rs(const struct spa_pod *latency, struct spa_process_latency_info *info) { return spa_process_latency_parse(latency, info); }
struct spa_pod * spa_process_latency_build_libspa_rs(struct spa_pod_builder *builder, uint32_t id, const struct spa_process_latency_info *info) { return spa_process_latency_build(builder, id, info); }
int spa_process_latency_info_add_libspa_rs(const struct spa_process_latency_info *process, struct spa_latency_info *info) { return spa_process_latency_info_add(process, info); }
int spa_process_latency_info_compare_libspa_rs(const struct spa_process_latency_info *a, const struct spa_process_latency_info *b) { return spa_process_latency_info_compare(a, b); }
int spa_format_audio_raw_ext_parse_libspa_rs(const struct spa_pod *format, struct spa_audio_info_raw *info, size_t size) { return spa_format_audio_raw_ext_parse(format, info, size); }
int spa_format_audio_raw_parse_libspa_rs(const struct spa_pod *format, struct spa_audio_info_raw *info) { return spa_format_audio_raw_parse(format, info); }
struct spa_pod * spa_format_audio_raw_ext_build_libspa_rs(struct spa_pod_builder *builder, uint32_t id, const struct spa_audio_info_raw *info, size_t size) { return spa_format_audio_raw_ext_build(builder, id, info, size); }
struct spa_pod * spa_format_audio_raw_build_libspa_rs(struct spa_pod_builder *builder, uint32_t id, const struct spa_audio_info_raw *info) { return spa_format_audio_raw_build(builder, id, info); }
int spa_format_audio_dsp_parse_libspa_rs(const struct spa_pod *format, struct spa_audio_info_dsp *info) { return spa_format_audio_dsp_parse(format, info); }
struct spa_pod * spa_format_audio_dsp_build_libspa_rs(struct spa_pod_builder *builder, uint32_t id, const struct spa_audio_info_dsp *info) { return spa_format_audio_dsp_build(builder, id, info); }
int spa_format_audio_iec958_parse_libspa_rs(const struct spa_pod *format, struct spa_audio_info_iec958 *info) { return spa_format_audio_iec958_parse(format, info); }
struct spa_pod * spa_format_audio_iec958_build_libspa_rs(struct spa_pod_builder *builder, uint32_t id, const struct spa_audio_info_iec958 *info) { return spa_format_audio_iec958_build(builder, id, info); }
int spa_format_audio_dsd_parse_libspa_rs(const struct spa_pod *format, struct spa_audio_info_dsd *info) { return spa_format_audio_dsd_parse(format, info); }
struct spa_pod * spa_format_audio_dsd_build_libspa_rs(struct spa_pod_builder *builder, uint32_t id, const struct spa_audio_info_dsd *info) { return spa_format_audio_dsd_build(builder, id, info); }
int spa_format_audio_mp3_parse_libspa_rs(const struct spa_pod *format, struct spa_audio_info_mp3 *info) { return spa_format_audio_mp3_parse(format, info); }
struct spa_pod * spa_format_audio_mp3_build_libspa_rs(struct spa_pod_builder *builder, uint32_t id, const struct spa_audio_info_mp3 *info) { return spa_format_audio_mp3_build(builder, id, info); }
int spa_format_audio_aac_parse_libspa_rs(const struct spa_pod *format, struct spa_audio_info_aac *info) { return spa_format_audio_aac_parse(format, info); }
struct spa_pod * spa_format_audio_aac_build_libspa_rs(struct spa_pod_builder *builder, uint32_t id, const struct spa_audio_info_aac *info) { return spa_format_audio_aac_build(builder, id, info); }
int spa_format_audio_vorbis_parse_libspa_rs(const struct spa_pod *format, struct spa_audio_info_vorbis *info) { return spa_format_audio_vorbis_parse(format, info); }
struct spa_pod * spa_format_audio_vorbis_build_libspa_rs(struct spa_pod_builder *builder, uint32_t id, const struct spa_audio_info_vorbis *info) { return spa_format_audio_vorbis_build(builder, id, info); }
int spa_format_audio_wma_parse_libspa_rs(const struct spa_pod *format, struct spa_audio_info_wma *info) { return spa_format_audio_wma_parse(format, info); }
struct spa_pod * spa_format_audio_wma_build_libspa_rs(struct spa_pod_builder *builder, uint32_t id, const struct spa_audio_info_wma *info) { return spa_format_audio_wma_build(builder, id, info); }
int spa_format_audio_ra_parse_libspa_rs(const struct spa_pod *format, struct spa_audio_info_ra *info) { return spa_format_audio_ra_parse(format, info); }
struct spa_pod * spa_format_audio_ra_build_libspa_rs(struct spa_pod_builder *builder, uint32_t id, const struct spa_audio_info_ra *info) { return spa_format_audio_ra_build(builder, id, info); }
int spa_format_audio_amr_parse_libspa_rs(const struct spa_pod *format, struct spa_audio_info_amr *info) { return spa_format_audio_amr_parse(format, info); }
struct spa_pod * spa_format_audio_amr_build_libspa_rs(struct spa_pod_builder *builder, uint32_t id, const struct spa_audio_info_amr *info) { return spa_format_audio_amr_build(builder, id, info); }
int spa_format_audio_alac_parse_libspa_rs(const struct spa_pod *format, struct spa_audio_info_alac *info) { return spa_format_audio_alac_parse(format, info); }
struct spa_pod * spa_format_audio_alac_build_libspa_rs(struct spa_pod_builder *builder, uint32_t id, const struct spa_audio_info_alac *info) { return spa_format_audio_alac_build(builder, id, info); }
int spa_format_audio_flac_parse_libspa_rs(const struct spa_pod *format, struct spa_audio_info_flac *info) { return spa_format_audio_flac_parse(format, info); }
struct spa_pod * spa_format_audio_flac_build_libspa_rs(struct spa_pod_builder *builder, uint32_t id, const struct spa_audio_info_flac *info) { return spa_format_audio_flac_build(builder, id, info); }
int spa_format_audio_ape_parse_libspa_rs(const struct spa_pod *format, struct spa_audio_info_ape *info) { return spa_format_audio_ape_parse(format, info); }
struct spa_pod * spa_format_audio_ape_build_libspa_rs(struct spa_pod_builder *builder, uint32_t id, const struct spa_audio_info_ape *info) { return spa_format_audio_ape_build(builder, id, info); }
int spa_format_audio_ac3_parse_libspa_rs(const struct spa_pod *format, struct spa_audio_info_ac3 *info) { return spa_format_audio_ac3_parse(format, info); }
struct spa_pod * spa_format_audio_ac3_build_libspa_rs(struct spa_pod_builder *builder, uint32_t id, const struct spa_audio_info_ac3 *info) { return spa_format_audio_ac3_build(builder, id, info); }
int spa_format_audio_eac3_parse_libspa_rs(const struct spa_pod *format, struct spa_audio_info_eac3 *info) { return spa_format_audio_eac3_parse(format, info); }
struct spa_pod * spa_format_audio_eac3_build_libspa_rs(struct spa_pod_builder *builder, uint32_t id, const struct spa_audio_info_eac3 *info) { return spa_format_audio_eac3_build(builder, id, info); }
int spa_format_audio_truehd_parse_libspa_rs(const struct spa_pod *format, struct spa_audio_info_truehd *info) { return spa_format_audio_truehd_parse(format, info); }
struct spa_pod * spa_format_audio_truehd_build_libspa_rs(struct spa_pod_builder *builder, uint32_t id, const struct spa_audio_info_truehd *info) { return spa_format_audio_truehd_build(builder, id, info); }
int spa_format_audio_dts_parse_libspa_rs(const struct spa_pod *format, struct spa_audio_info_dts *info) { return spa_format_audio_dts_parse(format, info); }
struct spa_pod * spa_format_audio_dts_build_libspa_rs(struct spa_pod_builder *builder, uint32_t id, const struct spa_audio_info_dts *info) { return spa_format_audio_dts_build(builder, id, info); }
int spa_format_audio_mpegh_parse_libspa_rs(const struct spa_pod *format, struct spa_audio_info_mpegh *info) { return spa_format_audio_mpegh_parse(format, info); }
struct spa_pod * spa_format_audio_mpegh_build_libspa_rs(struct spa_pod_builder *builder, uint32_t id, const struct spa_audio_info_mpegh *info) { return spa_format_audio_mpegh_build(builder, id, info); }
bool spa_format_audio_ext_valid_size_libspa_rs(uint32_t media_subtype, size_t size) { return spa_format_audio_ext_valid_size(media_subtype, size); }
int spa_format_audio_ext_parse_libspa_rs(const struct spa_pod *format, struct spa_audio_info *info, size_t size) { return spa_format_audio_ext_parse(format, info, size); }
int spa_format_audio_parse_libspa_rs(const struct spa_pod *format, struct spa_audio_info *info) { return spa_format_audio_parse(format, info); }
struct spa_pod * spa_format_audio_ext_build_libspa_rs(struct spa_pod_builder *builder, uint32_t id, const struct spa_audio_info *info, size_t size) { return spa_format_audio_ext_build(builder, id, info, size); }
struct spa_pod * spa_format_audio_build_libspa_rs(struct spa_pod_builder *builder, uint32_t id, const struct spa_audio_info *info) { return spa_format_audio_build(builder, id, info); }
int spa_format_video_raw_parse_libspa_rs(const struct spa_pod *format, struct spa_video_info_raw *info) { return spa_format_video_raw_parse(format, info); }
struct spa_pod * spa_format_video_raw_build_libspa_rs(struct spa_pod_builder *builder, uint32_t id, const struct spa_video_info_raw *info) { return spa_format_video_raw_build(builder, id, info); }
bool spa_format_video_is_rgb_libspa_rs(enum spa_video_format format) { return spa_format_video_is_rgb(format); }
int spa_format_video_dsp_parse_libspa_rs(const struct spa_pod *format, struct spa_video_info_dsp *info) { return spa_format_video_dsp_parse(format, info); }
struct spa_pod * spa_format_video_dsp_build_libspa_rs(struct spa_pod_builder *builder, uint32_t id, const struct spa_video_info_dsp *info) { return spa_format_video_dsp_build(builder, id, info); }
int spa_format_video_h264_parse_libspa_rs(const struct spa_pod *format, struct spa_video_info_h264 *info) { return spa_format_video_h264_parse(format, info); }
struct spa_pod * spa_format_video_h264_build_libspa_rs(struct spa_pod_builder *builder, uint32_t id, const struct spa_video_info_h264 *info) { return spa_format_video_h264_build(builder, id, info); }
int spa_format_video_h265_parse_libspa_rs(const struct spa_pod *format, struct spa_video_info_h265 *info) { return spa_format_video_h265_parse(format, info); }
struct spa_pod * spa_format_video_h265_build_libspa_rs(struct spa_pod_builder *builder, uint32_t id, const struct spa_video_info_h265 *info) { return spa_format_video_h265_build(builder, id, info); }
int spa_format_video_mjpg_parse_libspa_rs(const struct spa_pod *format, struct spa_video_info_mjpg *info) { return spa_format_video_mjpg_parse(format, info); }
struct spa_pod * spa_format_video_mjpg_build_libspa_rs(struct spa_pod_builder *builder, uint32_t id, const struct spa_video_info_mjpg *info) { return spa_format_video_mjpg_build(builder, id, info); }
int spa_format_video_parse_libspa_rs(const struct spa_pod *format, struct spa_video_info *info) { return spa_format_video_parse(format, info); }
struct spa_pod * spa_format_video_build_libspa_rs(struct spa_pod_builder *builder, uint32_t id, const struct spa_video_info *info) { return spa_format_video_build(builder, id, info); }
int spa_pod_compare_value_libspa_rs(uint32_t type, const void *r1, const void *r2, uint32_t size) { return spa_pod_compare_value(type, r1, r2, size); }
int spa_pod_memcmp_libspa_rs(const struct spa_pod *a, const struct spa_pod *b) { return spa_pod_memcmp(a, b); }
int spa_pod_compare_libspa_rs(const struct spa_pod *pod1, const struct spa_pod *pod2) { return spa_pod_compare(pod1, pod2); }
int spa_pod_compare_is_compatible_flags_libspa_rs(uint32_t type, const void *r1, const void *r2, uint32_t size) { return spa_pod_compare_is_compatible_flags(type, r1, r2, size); }
int spa_pod_compare_is_step_of_libspa_rs(uint32_t type, const void *r1, const void *r2, uint32_t size) { return spa_pod_compare_is_step_of(type, r1, r2, size); }
int spa_pod_compare_is_in_range_libspa_rs(uint32_t type, const void *v, const void *min, const void *max, const void *step, uint32_t size) { return spa_pod_compare_is_in_range(type, v, min, max, step, size); }
int spa_pod_compare_is_valid_choice_libspa_rs(uint32_t type, uint32_t size, const void *val, const void *vals, uint32_t n_vals, uint32_t choice) { return spa_pod_compare_is_valid_choice(type, size, val, vals, n_vals, choice); }
int spa_pod_dynamic_builder_overflow_libspa_rs(void *data, uint32_t size) { return spa_pod_dynamic_builder_overflow(data, size); }
void spa_pod_dynamic_builder_init_libspa_rs(struct spa_pod_dynamic_builder *builder, void *data, uint32_t size, uint32_t extend) { spa_pod_dynamic_builder_init(builder, data, size, extend); }
void spa_pod_dynamic_builder_continue_libspa_rs(struct spa_pod_dynamic_builder *builder, struct spa_pod_builder *b) { spa_pod_dynamic_builder_continue(builder, b); }
void spa_pod_dynamic_builder_clean_libspa_rs(struct spa_pod_dynamic_builder *builder) { spa_pod_dynamic_builder_clean(builder); }
int spa_pod_filter_flags_value_libspa_rs(struct spa_pod_builder *b, uint32_t type, const void *r1, const void *r2, uint32_t size) { return spa_pod_filter_flags_value(b, type, r1, r2, size); }
int spa_pod_filter_prop_libspa_rs(struct spa_pod_builder *b, const struct spa_pod_prop *p1, const struct spa_pod_prop *p2) { return spa_pod_filter_prop(b, p1, p2); }
int spa_pod_filter_part_libspa_rs(struct spa_pod_builder *b, const struct spa_pod *pod, uint32_t pod_size, const struct spa_pod *filter, uint32_t filter_size) { return spa_pod_filter_part(b, pod, pod_size, filter, filter_size); }
int spa_pod_filter_libspa_rs(struct spa_pod_builder *b, struct spa_pod **result, const struct spa_pod *pod, const struct spa_pod *filter) { return spa_pod_filter(b, result, pod, filter); }
int spa_pod_filter_object_make_libspa_rs(struct spa_pod_object *pod) { return spa_pod_filter_object_make(pod); }
int spa_pod_filter_make_libspa_rs(struct spa_pod *pod) { return spa_pod_filter_make(pod); }
const char * spa_cpu_vm_type_to_string_libspa_rs(uint32_t vm_type) { return spa_cpu_vm_type_to_string(vm_type); }
uint32_t spa_cpu_get_flags_libspa_rs(struct spa_cpu *c) { return spa_cpu_get_flags(c); }
int spa_cpu_force_flags_libspa_rs(struct spa_cpu *c, uint32_t flags) { return spa_cpu_force_flags(c, flags); }
uint32_t spa_cpu_get_count_libspa_rs(struct spa_cpu *c) { return spa_cpu_get_count(c); }
uint32_t spa_cpu_get_max_align_libspa_rs(struct spa_cpu *c) { return spa_cpu_get_max_align(c); }
uint32_t spa_cpu_get_vm_type_libspa_rs(struct spa_cpu *c) { return spa_cpu_get_vm_type(c); }
int spa_cpu_zero_denormals_libspa_rs(struct spa_cpu *c, bool enable) { return spa_cpu_zero_denormals(c, enable); }
ssize_t spa_system_read_libspa_rs(struct spa_system *object, int fd, void *buf, size_t count) { return spa_system_read(object, fd, buf, count); }
ssize_t spa_system_write_libspa_rs(struct spa_system *object, int fd, const void *buf, size_t count) { return spa_system_write(object, fd, buf, count); }
int spa_system_close_libspa_rs(struct spa_system *object, int fd) { return spa_system_close(object, fd); }
int spa_system_clock_gettime_libspa_rs(struct spa_system *object, int clockid, struct timespec *value) { return spa_system_clock_gettime(object, clockid, value); }
int spa_system_clock_getres_libspa_rs(struct spa_system *object, int clockid, struct timespec *res) { return spa_system_clock_getres(object, clockid, res); }
int spa_system_pollfd_create_libspa_rs(struct spa_system *object, int flags) { return spa_system_pollfd_create(object, flags); }
int spa_system_pollfd_add_libspa_rs(struct spa_system *object, int pfd, int fd, uint32_t events, void *data) { return spa_system_pollfd_add(object, pfd, fd, events, data); }
int spa_system_pollfd_mod_libspa_rs(struct spa_system *object, int pfd, int fd, uint32_t events, void *data) { return spa_system_pollfd_mod(object, pfd, fd, events, data); }
int spa_system_pollfd_del_libspa_rs(struct spa_system *object, int pfd, int fd) { return spa_system_pollfd_del(object, pfd, fd); }
int spa_system_pollfd_wait_libspa_rs(struct spa_system *object, int pfd, struct spa_poll_event *ev, int n_ev, int timeout) { return spa_system_pollfd_wait(object, pfd, ev, n_ev, timeout); }
int spa_system_timerfd_create_libspa_rs(struct spa_system *object, int clockid, int flags) { return spa_system_timerfd_create(object, clockid, flags); }
int spa_system_timerfd_settime_libspa_rs(struct spa_system *object, int fd, int flags, const struct itimerspec *new_value, struct itimerspec *old_value) { return spa_system_timerfd_settime(object, fd, flags, new_value, old_value); }
int spa_system_timerfd_gettime_libspa_rs(struct spa_system *object, int fd, struct itimerspec *curr_value) { return spa_system_timerfd_gettime(object, fd, curr_value); }
int spa_system_timerfd_read_libspa_rs(struct spa_system *object, int fd, uint64_t *expirations) { return spa_system_timerfd_read(object, fd, expirations); }
int spa_system_eventfd_create_libspa_rs(struct spa_system *object, int flags) { return spa_system_eventfd_create(object, flags); }
int spa_system_eventfd_write_libspa_rs(struct spa_system *object, int fd, uint64_t count) { return spa_system_eventfd_write(object, fd, count); }
int spa_system_eventfd_read_libspa_rs(struct spa_system *object, int fd, uint64_t *count) { return spa_system_eventfd_read(object, fd, count); }
int spa_system_signalfd_create_libspa_rs(struct spa_system *object, int signal, int flags) { return spa_system_signalfd_create(object, signal, flags); }
int spa_system_signalfd_read_libspa_rs(struct spa_system *object, int fd, int *signal) { return spa_system_signalfd_read(object, fd, signal); }
int spa_loop_add_source_libspa_rs(struct spa_loop *object, struct spa_source *source) { return spa_loop_add_source(object, source); }
int spa_loop_update_source_libspa_rs(struct spa_loop *object, struct spa_source *source) { return spa_loop_update_source(object, source); }
int spa_loop_remove_source_libspa_rs(struct spa_loop *object, struct spa_source *source) { return spa_loop_remove_source(object, source); }
int spa_loop_invoke_libspa_rs(struct spa_loop *object, spa_invoke_func_t func, uint32_t seq, const void *data, size_t size, bool block, void *user_data) { return spa_loop_invoke(object, func, seq, data, size, block, user_data); }
int spa_loop_locked_libspa_rs(struct spa_loop *object, spa_invoke_func_t func, uint32_t seq, const void *data, size_t size, void *user_data) { return spa_loop_locked(object, func, seq, data, size, user_data); }
void spa_loop_control_hook_before_libspa_rs(struct spa_hook_list *l) { spa_loop_control_hook_before(l); }
void spa_loop_control_hook_after_libspa_rs(struct spa_hook_list *l) { spa_loop_control_hook_after(l); }
int spa_loop_control_get_fd_libspa_rs(struct spa_loop_control *object) { return spa_loop_control_get_fd(object); }
void spa_loop_control_add_hook_libspa_rs(struct spa_loop_control *object, struct spa_hook *hook, const struct spa_loop_control_hooks *hooks, void *data) { spa_loop_control_add_hook(object, hook, hooks, data); }
void spa_loop_control_enter_libspa_rs(struct spa_loop_control *object) { spa_loop_control_enter(object); }
void spa_loop_control_leave_libspa_rs(struct spa_loop_control *object) { spa_loop_control_leave(object); }
int spa_loop_control_iterate_libspa_rs(struct spa_loop_control *object, int timeout) { return spa_loop_control_iterate(object, timeout); }
int spa_loop_control_iterate_fast_libspa_rs(struct spa_loop_control *object, int timeout) { return spa_loop_control_iterate_fast(object, timeout); }
int spa_loop_control_check_libspa_rs(struct spa_loop_control *object) { return spa_loop_control_check(object); }
int spa_loop_control_lock_libspa_rs(struct spa_loop_control *object) { return spa_loop_control_lock(object); }
int spa_loop_control_unlock_libspa_rs(struct spa_loop_control *object) { return spa_loop_control_unlock(object); }
int spa_loop_control_get_time_libspa_rs(struct spa_loop_control *object, struct timespec *abstime, int64_t timeout) { return spa_loop_control_get_time(object, abstime, timeout); }
int spa_loop_control_wait_libspa_rs(struct spa_loop_control *object, const struct timespec *abstime) { return spa_loop_control_wait(object, abstime); }
int spa_loop_control_signal_libspa_rs(struct spa_loop_control *object, bool wait_for_accept) { return spa_loop_control_signal(object, wait_for_accept); }
int spa_loop_control_accept_libspa_rs(struct spa_loop_control *object) { return spa_loop_control_accept(object); }
struct spa_source * spa_loop_utils_add_io_libspa_rs(struct spa_loop_utils *object, int fd, uint32_t mask, bool close, spa_source_io_func_t func, void *data) { return spa_loop_utils_add_io(object, fd, mask, close, func, data); }
int spa_loop_utils_update_io_libspa_rs(struct spa_loop_utils *object, struct spa_source *source, uint32_t mask) { return spa_loop_utils_update_io(object, source, mask); }
struct spa_source * spa_loop_utils_add_idle_libspa_rs(struct spa_loop_utils *object, bool enabled, spa_source_idle_func_t func, void *data) { return spa_loop_utils_add_idle(object, enabled, func, data); }
int spa_loop_utils_enable_idle_libspa_rs(struct spa_loop_utils *object, struct spa_source *source, bool enabled) { return spa_loop_utils_enable_idle(object, source, enabled); }
struct spa_source * spa_loop_utils_add_event_libspa_rs(struct spa_loop_utils *object, spa_source_event_func_t func, void *data) { return spa_loop_utils_add_event(object, func, data); }
int spa_loop_utils_signal_event_libspa_rs(struct spa_loop_utils *object, struct spa_source *source) { return spa_loop_utils_signal_event(object, source); }
struct spa_source * spa_loop_utils_add_timer_libspa_rs(struct spa_loop_utils *object, spa_source_timer_func_t func, void *data) { return spa_loop_utils_add_timer(object, func, data); }
int spa_loop_utils_update_timer_libspa_rs(struct spa_loop_utils *object, struct spa_source *source, struct timespec *value, struct timespec *interval, bool absolute) { return spa_loop_utils_update_timer(object, source, value, interval, absolute); }
struct spa_source * spa_loop_utils_add_signal_libspa_rs(struct spa_loop_utils *object, int signal_number, spa_source_signal_func_t func, void *data) { return spa_loop_utils_add_signal(object, signal_number, func, data); }
void spa_loop_utils_destroy_source_libspa_rs(struct spa_loop_utils *object, struct spa_source *source) { spa_loop_utils_destroy_source(object, source); }
void * spa_dbus_connection_get_libspa_rs(struct spa_dbus_connection *conn) { return spa_dbus_connection_get(conn); }
void spa_dbus_connection_destroy_libspa_rs(struct spa_dbus_connection *conn) { spa_dbus_connection_destroy(conn); }
void spa_dbus_connection_add_listener_libspa_rs(struct spa_dbus_connection *conn, struct spa_hook *listener, const struct spa_dbus_connection_events *events, void *data) { spa_dbus_connection_add_listener(conn, listener, events, data); }
struct spa_dbus_connection * spa_dbus_get_connection_libspa_rs(struct spa_dbus *dbus, enum spa_dbus_type type) { return spa_dbus_get_connection(dbus, type); }
const char * spa_i18n_text_libspa_rs(struct spa_i18n *i18n, const char *msgid) { return spa_i18n_text(i18n, msgid); }
const char * spa_i18n_ntext_libspa_rs(struct spa_i18n *i18n, const char *msgid, const char *msgid_plural, unsigned long n) { return spa_i18n_ntext(i18n, msgid, msgid_plural, n); }
void spa_log_topic_init_libspa_rs(struct spa_log *log, struct spa_log_topic *topic) { spa_log_topic_init(log, topic); }
bool spa_log_level_topic_enabled_libspa_rs(const struct spa_log *log, const struct spa_log_topic *topic, enum spa_log_level level) { return spa_log_level_topic_enabled(log, topic, level); }
void spa_log_logtv_libspa_rs(struct spa_log *l, enum spa_log_level level, const struct spa_log_topic *topic, const char *file, int line, const char *func, const char *fmt, va_list args) { spa_log_logtv(l, level, topic, file, line, func, fmt, args); }
void spa_log_impl_logtv_libspa_rs(void *object, enum spa_log_level level, const struct spa_log_topic *topic, const char *file, int line, const char *func, const char *fmt, va_list args) { spa_log_impl_logtv(object, level, topic, file, line, func, fmt, args); }
void spa_log_impl_logv_libspa_rs(void *object, enum spa_log_level level, const char *file, int line, const char *func, const char *fmt, va_list args) { spa_log_impl_logv(object, level, file, line, func, fmt, args); }
void spa_log_impl_topic_init_libspa_rs(void *object, struct spa_log_topic *topic) { spa_log_impl_topic_init(object, topic); }
struct spa_handle * spa_plugin_loader_load_libspa_rs(struct spa_plugin_loader *loader, const char *factory_name, const struct spa_dict *info) { return spa_plugin_loader_load(loader, factory_name, info); }
int spa_plugin_loader_unload_libspa_rs(struct spa_plugin_loader *loader, struct spa_handle *handle) { return spa_plugin_loader_unload(loader, handle); }
int spa_handle_get_interface_libspa_rs(struct spa_handle *object, const char *type, void **iface) { return spa_handle_get_interface(object, type, iface); }
int spa_handle_clear_libspa_rs(struct spa_handle *object) { return spa_handle_clear(object); }
void * spa_support_find_libspa_rs(const struct spa_support *support, uint32_t n_support, const char *type) { return spa_support_find(support, n_support, type); }
size_t spa_handle_factory_get_size_libspa_rs(const struct spa_handle_factory *object, const struct spa_dict *params) { return spa_handle_factory_get_size(object, params); }
int spa_handle_factory_init_libspa_rs(const struct spa_handle_factory *object, struct spa_handle *handle, const struct spa_dict *info, const struct spa_support *support, uint32_t n_support) { return spa_handle_factory_init(object, handle, info, support, n_support); }
int spa_handle_factory_enum_interface_info_libspa_rs(const struct spa_handle_factory *object, const struct spa_interface_info **info, uint32_t *index) { return spa_handle_factory_enum_interface_info(object, info, index); }
struct spa_thread * spa_thread_utils_create_libspa_rs(struct spa_thread_utils *o, const struct spa_dict *props, void * (*start_routine) (void *), void *arg) { return spa_thread_utils_create(o, props, start_routine, arg); }
int spa_thread_utils_join_libspa_rs(struct spa_thread_utils *o, struct spa_thread *thread, void **retval) { return spa_thread_utils_join(o, thread, retval); }
int spa_thread_utils_get_rt_range_libspa_rs(struct spa_thread_utils *o, const struct spa_dict *props, int *min, int *max) { return spa_thread_utils_get_rt_range(o, props, min, max); }
int spa_thread_utils_acquire_rt_libspa_rs(struct spa_thread_utils *o, struct spa_thread *thread, int priority) { return spa_thread_utils_acquire_rt(o, thread, priority); }
int spa_thread_utils_drop_rt_libspa_rs(struct spa_thread_utils *o, struct spa_thread *thread) { return spa_thread_utils_drop_rt(o, thread); }
void spa_json_init_libspa_rs(struct spa_json *iter, const char *data, size_t size) { spa_json_init(iter, data, size); }
void spa_json_init_relax_libspa_rs(struct spa_json *iter, char type, const char *data, size_t size) { spa_json_init_relax(iter, type, data, size); }
void spa_json_enter_libspa_rs(struct spa_json *iter, struct spa_json *sub) { spa_json_enter(iter, sub); }
void spa_json_save_libspa_rs(struct spa_json *iter, struct spa_json *save) { spa_json_save(iter, save); }
void spa_json_start_libspa_rs(struct spa_json *iter, struct spa_json *sub, const char *pos) { spa_json_start(iter, sub, pos); }
int spa_json_next_libspa_rs(struct spa_json *iter, const char **value) { return spa_json_next(iter, value); }
bool spa_json_get_error_libspa_rs(struct spa_json *iter, const char *start, struct spa_error_location *loc) { return spa_json_get_error(iter, start, loc); }
int spa_json_is_container_libspa_rs(const char *val, int len) { return spa_json_is_container(val, len); }
int spa_json_is_object_libspa_rs(const char *val, int len) { return spa_json_is_object(val, len); }
bool spa_json_is_array_libspa_rs(const char *val, int len) { return spa_json_is_array(val, len); }
bool spa_json_is_null_libspa_rs(const char *val, int len) { return spa_json_is_null(val, len); }
int spa_json_parse_float_libspa_rs(const char *val, int len, float *result) { return spa_json_parse_float(val, len, result); }
bool spa_json_is_float_libspa_rs(const char *val, int len) { return spa_json_is_float(val, len); }
char * spa_json_format_float_libspa_rs(char *str, int size, float val) { return spa_json_format_float(str, size, val); }
int spa_json_parse_int_libspa_rs(const char *val, int len, int *result) { return spa_json_parse_int(val, len, result); }
bool spa_json_is_int_libspa_rs(const char *val, int len) { return spa_json_is_int(val, len); }
bool spa_json_is_true_libspa_rs(const char *val, int len) { return spa_json_is_true(val, len); }
bool spa_json_is_false_libspa_rs(const char *val, int len) { return spa_json_is_false(val, len); }
bool spa_json_is_bool_libspa_rs(const char *val, int len) { return spa_json_is_bool(val, len); }
int spa_json_parse_bool_libspa_rs(const char *val, int len, bool *result) { return spa_json_parse_bool(val, len, result); }
bool spa_json_is_string_libspa_rs(const char *val, int len) { return spa_json_is_string(val, len); }
int spa_json_parse_hex_libspa_rs(const char *p, int num, uint32_t *res) { return spa_json_parse_hex(p, num, res); }
int spa_json_parse_stringn_libspa_rs(const char *val, int len, char *result, int maxlen) { return spa_json_parse_stringn(val, len, result, maxlen); }
int spa_json_parse_string_libspa_rs(const char *val, int len, char *result) { return spa_json_parse_string(val, len, result); }
int spa_json_encode_string_libspa_rs(char *str, int size, const char *val) { return spa_json_encode_string(str, size, val); }
int spa_json_begin_libspa_rs(struct spa_json *iter, const char *data, size_t size, const char **val) { return spa_json_begin(iter, data, size, val); }
int spa_json_get_float_libspa_rs(struct spa_json *iter, float *res) { return spa_json_get_float(iter, res); }
int spa_json_get_int_libspa_rs(struct spa_json *iter, int *res) { return spa_json_get_int(iter, res); }
int spa_json_get_bool_libspa_rs(struct spa_json *iter, bool *res) { return spa_json_get_bool(iter, res); }
int spa_json_get_string_libspa_rs(struct spa_json *iter, char *res, int maxlen) { return spa_json_get_string(iter, res, maxlen); }
int spa_json_enter_container_libspa_rs(struct spa_json *iter, struct spa_json *sub, char type) { return spa_json_enter_container(iter, sub, type); }
int spa_json_begin_container_libspa_rs(struct spa_json *iter, const char *data, size_t size, char type, bool relax) { return spa_json_begin_container(iter, data, size, type, relax); }
int spa_json_container_len_libspa_rs(struct spa_json *iter, const char *value, int len) { return spa_json_container_len(iter, value, len); }
int spa_json_enter_object_libspa_rs(struct spa_json *iter, struct spa_json *sub) { return spa_json_enter_object(iter, sub); }
int spa_json_begin_object_relax_libspa_rs(struct spa_json *iter, const char *data, size_t size) { return spa_json_begin_object_relax(iter, data, size); }
int spa_json_begin_object_libspa_rs(struct spa_json *iter, const char *data, size_t size) { return spa_json_begin_object(iter, data, size); }
int spa_json_object_next_libspa_rs(struct spa_json *iter, char *key, int maxkeylen, const char **value) { return spa_json_object_next(iter, key, maxkeylen, value); }
int spa_json_object_find_libspa_rs(struct spa_json *iter, const char *key, const char **value) { return spa_json_object_find(iter, key, value); }
int spa_json_str_object_find_libspa_rs(const char *obj, size_t obj_len, const char *key, char *value, size_t maxlen) { return spa_json_str_object_find(obj, obj_len, key, value, maxlen); }
int spa_json_enter_array_libspa_rs(struct spa_json *iter, struct spa_json *sub) { return spa_json_enter_array(iter, sub); }
int spa_json_begin_array_relax_libspa_rs(struct spa_json *iter, const char *data, size_t size) { return spa_json_begin_array_relax(iter, data, size); }
int spa_json_begin_array_libspa_rs(struct spa_json *iter, const char *data, size_t size) { return spa_json_begin_array(iter, data, size); }
int spa_json_str_array_uint32_libspa_rs(const char *arr, size_t arr_len, uint32_t *values, size_t max) { return spa_json_str_array_uint32(arr, arr_len, values, max); }
const char * spa_strerror_libspa_rs(int err) { return spa_strerror(err); }
void spa_ringbuffer_init_libspa_rs(struct spa_ringbuffer *rbuf) { spa_ringbuffer_init(rbuf); }
void spa_ringbuffer_set_avail_libspa_rs(struct spa_ringbuffer *rbuf, uint32_t size) { spa_ringbuffer_set_avail(rbuf, size); }
int32_t spa_ringbuffer_get_read_index_libspa_rs(struct spa_ringbuffer *rbuf, uint32_t *index) { return spa_ringbuffer_get_read_index(rbuf, index); }
void spa_ringbuffer_read_data_libspa_rs(struct spa_ringbuffer *rbuf, const void *buffer, uint32_t size, uint32_t offset, void *data, uint32_t len) { spa_ringbuffer_read_data(rbuf, buffer, size, offset, data, len); }
void spa_ringbuffer_read_update_libspa_rs(struct spa_ringbuffer *rbuf, int32_t index) { spa_ringbuffer_read_update(rbuf, index); }
int32_t spa_ringbuffer_get_write_index_libspa_rs(struct spa_ringbuffer *rbuf, uint32_t *index) { return spa_ringbuffer_get_write_index(rbuf, index); }
void spa_ringbuffer_write_data_libspa_rs(struct spa_ringbuffer *rbuf, void *buffer, uint32_t size, uint32_t offset, const void *data, uint32_t len) { spa_ringbuffer_write_data(rbuf, buffer, size, offset, data, len); }
void spa_ringbuffer_write_update_libspa_rs(struct spa_ringbuffer *rbuf, int32_t index) { spa_ringbuffer_write_update(rbuf, index); }
