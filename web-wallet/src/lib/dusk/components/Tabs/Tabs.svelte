<svelte:options immutable={true} />

<script>
  import { mdiChevronLeft, mdiChevronRight } from "@mdi/js";
  import { createEventDispatcher, onMount } from "svelte";
  import { writable } from "svelte/store";
  import { isGTE, isLTE } from "lamb";

  import { Button, Icon } from "$lib/dusk/components";
  import { lerp } from "$lib/dusk/math";
  import { makeClassName } from "$lib/dusk/string";

  import "./Tabs.css";

  /** @type {String | Undefined} */
  export let className = undefined;

  /** @type {TabItem[]} */
  export let items;

  /** @type {String | Undefined} */
  export let selectedTab = undefined;

  /** @type {HTMLUListElement} */
  let tabsList;

  /** @type {Number} */
  let rafID = 0;

  let scrollLeftStart = 0;
  let touchStartX = 0;

  const dispatch = createEventDispatcher();

  const scrollStatus = writable({
    canScroll: false,
    canScrollLeft: false,
    canScrollRight: false,
  });

  /** @type {ScrollIntoViewOptions} */
  const smoothScrollOptions = {
    behavior: "smooth",
    block: "nearest",
    inline: "nearest",
  };

  /** @param {"left" | "right"} side */
  function isTabSideVisible(side) {
    const tabsListRect = tabsList.getBoundingClientRect();
    const tolerance = 5;
    const checkSide =
      side === "left"
        ? isGTE(tabsListRect.left - tolerance)
        : isLTE(tabsListRect.right + tolerance);

    /** @param {HTMLLIElement} tab */
    return (tab) => checkSide(tab.getBoundingClientRect()[side]);
  }

  /** @type {import("svelte/elements").TouchEventHandler<HTMLUListElement>} */
  function handleTouchMove(event) {
    const scrollX = tabsList.scrollLeft;
    const deltaX = event.targetTouches[0].clientX - touchStartX;
    const amount = lerp(deltaX, scrollX - scrollLeftStart, 0.4);

    tabsList.scrollBy(-amount, 0);
  }

  /** @type {import("svelte/elements").TouchEventHandler<HTMLUListElement>} */
  function handleTouchStart(event) {
    scrollLeftStart = tabsList.scrollLeft;
    touchStartX = event.targetTouches[0].clientX;
  }

  /** @type {import("svelte/elements").WheelEventHandler<HTMLUListElement>} */
  function handleWheel(event) {
    tabsList.scrollBy(event.deltaX, 0);
  }

  // @ts-ignore
  function handleScrollButtonClick(event) {
    /** @type {NodeListOf<HTMLLIElement>} */
    const tabs = tabsList.querySelectorAll("[role='tab']");
    const step = event.currentTarget.matches(
      ".dusk-tab-scroll-button:first-of-type"
    )
      ? -1
      : 1;
    const isTabFullyVisible = isTabSideVisible(step === 1 ? "right" : "left");

    let loops = tabs.length;
    let idx = step === 1 ? 0 : loops - 1;

    for (; loops--; idx += step) {
      if (!isTabFullyVisible(tabs[idx])) {
        tabs[idx].scrollIntoView(smoothScrollOptions);
        break;
      }
    }
  }

  // @ts-ignore
  function handleScrollButtonMouseDown(event) {
    if (event.buttons === 1) {
      const amount = event.currentTarget.matches(
        ".dusk-tab-scroll-button:first-of-type"
      )
        ? -5
        : 5;

      keepScrollingTabsBy(amount);
    }
  }

  function handleScrollButtonMouseUp() {
    cancelAnimationFrame(rafID);
  }

  /** @type {import("svelte/elements").UIEventHandler<HTMLLIElement>} */
  function handleTabClick(event) {
    const clickedID = event.currentTarget.dataset.tabid;

    if (selectedTab !== clickedID) {
      selectedTab = clickedID;
      dispatch("change", clickedID);
    }
  }

  /** @type {import("svelte/elements").UIEventHandler<HTMLLIElement>} */
  function handleTabFocusin(event) {
    event.currentTarget.scrollIntoView(smoothScrollOptions);
  }

  /** @type {import("svelte/elements").KeyboardEventHandler<HTMLLIElement>} */
  function handleTabKeyDown(event) {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();

      handleTabClick(event);
    }
  }

  /** @param {Number} amount */
  function keepScrollingTabsBy(amount) {
    const { canScrollLeft, canScrollRight } = $scrollStatus;

    tabsList.scrollBy(amount, 0);

    if ((canScrollLeft && amount < 0) || (canScrollRight && amount > 0)) {
      rafID = requestAnimationFrame(() => keepScrollingTabsBy(amount));
    }
  }

  function updateScrollStatus() {
    const { clientWidth = 0, scrollLeft = 0, scrollWidth = 0 } = tabsList;

    const canScroll = scrollWidth > clientWidth;
    const maxScroll = scrollWidth - clientWidth;

    scrollStatus.set({
      canScroll,
      canScrollLeft: canScroll && scrollLeft > 0,
      canScrollRight: canScroll && scrollLeft < maxScroll,
    });
  }

  onMount(() => {
    const resizeObserver = new ResizeObserver(() => {
      const tab = tabsList.querySelector(`[data-tabid="${selectedTab}"]`);

      tab &&
        tab.scrollIntoView({
          behavior: "instant",
          block: "nearest",
          inline: "nearest",
        });

      updateScrollStatus();
    });

    tabsList.scrollTo(0, 0);
    resizeObserver.observe(tabsList);

    return () => resizeObserver.disconnect();
  });

  $: ({ canScroll, canScrollLeft, canScrollRight } = $scrollStatus);
</script>

<div {...$$restProps} class={makeClassName(["dusk-tabs", className])}>
  <Button
    className="dusk-tab-scroll-button"
    disabled={!canScrollLeft}
    hidden={!canScroll}
    icon={{ path: mdiChevronLeft }}
    on:click={handleScrollButtonClick}
    on:mousedown={handleScrollButtonMouseDown}
    on:mouseup={handleScrollButtonMouseUp}
    tabindex="-1"
    variant="quaternary"
  />
  <ul
    bind:this={tabsList}
    class="dusk-tabs-list"
    on:scroll={updateScrollStatus}
    on:touchmove|preventDefault={handleTouchMove}
    on:touchstart={handleTouchStart}
    on:wheel|preventDefault={handleWheel}
    role="tablist"
  >
    {#each items as item (item.id)}
      {@const { icon, id, label } = item}
      <li
        aria-selected={id === selectedTab}
        class={`dusk-tab-item${id === selectedTab ? " dusk-tab-item__selected" : ""}`}
        data-tabid={id}
        on:click={handleTabClick}
        on:focusin={handleTabFocusin}
        on:keydown={handleTabKeyDown}
        role="tab"
        tabindex="0"
      >
        {#if icon?.position === "after"}
          {#if label}
            <span class="dusk-tab-label">{label}</span>
          {/if}
          <Icon path={icon.path} />
        {:else if icon}
          <Icon path={icon.path} />
          {#if label}
            <span class="dusk-tab-label">{label}</span>
          {/if}
        {:else}
          <span class="dusk-tab-label">{label ?? id}</span>
        {/if}
      </li>
    {/each}
  </ul>
  <Button
    className="dusk-tab-scroll-button"
    disabled={!canScrollRight}
    hidden={!canScroll}
    icon={{ path: mdiChevronRight }}
    on:click={handleScrollButtonClick}
    on:mousedown={handleScrollButtonMouseDown}
    on:mouseup={handleScrollButtonMouseUp}
    tabindex="-1"
    variant="quaternary"
  />
</div>
