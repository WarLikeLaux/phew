<div x-data="{ open: false, label: '{{ label }}' }">
    <template x-if="open">
        <span>{{ title }}</span>
    </template>
    <script type="text/template" id="row-template">
        <% _.each(items, function (item) { %>
        <article class="row">
        <h3>{{ item.title }}</h3>
        <p><%= item.description %></p>
        </article>
        <% }) %>
    </script>
</div>
